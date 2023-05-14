use chrono::{DateTime, Local, Timelike};
use image::{ImageBuffer, Rgb, RgbImage};
use json::JsonValue;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::prelude::*;

fn main() -> std::io::Result<()> {
    let now: DateTime<Local> = Local::now();
    let seconds = now.num_seconds_from_midnight();

    let file = File::open("scenes/April-Clear.json");
    let mut contents = String::new();
    file?.read_to_string(&mut contents)?;

    let parsed = json::parse(&contents).unwrap();

    let base: HashMap<&str, &JsonValue> = parsed["base"].entries().collect();

    let width: u32 = base["width"].as_u32().unwrap();
    let height: u32 = base["height"].as_u32().unwrap();
    let pixels: Vec<u8> = base["pixels"]
        .members()
        .map(|x| x.as_u8().unwrap())
        .collect();

    let palettes = parsed["palettes"].clone();

    let mut image: RgbImage = ImageBuffer::new(width, height);

    let timeline: BTreeMap<u32, &str> = parsed["timeline"]
        .entries()
        .map(|(time, palette)| (time.parse::<u32>().unwrap(), palette.as_str().unwrap()))
        .collect();

    let mut last_time = timeline.keys().last().unwrap();
    for (time, _) in timeline.iter() {
        if time > &seconds {
            break;
        }
        last_time = time;
    }

    let palette = &palettes[*timeline.get(last_time).unwrap()];

    let colors: Vec<[u8; 3]> = palette["colors"]
        .members()
        .map(|x| {
            [
                x[0].as_u8().unwrap(),
                x[1].as_u8().unwrap(),
                x[2].as_u8().unwrap(),
            ]
        })
        .collect();

    for (x, y, pixel) in image.enumerate_pixels_mut() {
        let index = (y * width + x) as usize;
        let color = colors[pixels[index] as usize];
        *pixel = Rgb(color);
    }

    image.save("test.png").unwrap();

    Ok(())
}

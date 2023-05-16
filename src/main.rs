use chrono::{DateTime, Local, Timelike};
use json::JsonValue;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::TextureCreator;
use sdl2::video::WindowContext;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::prelude::*;
use std::time::Duration;

fn main() -> std::io::Result<()> {
    let now: DateTime<Local> = Local::now();
    let seconds: u32 = now.num_seconds_from_midnight();

    let file: Result<File, std::io::Error> = File::open("scenes/April-Clear.json");
    let mut contents: String = String::new();
    file?.read_to_string(&mut contents)?;

    let parsed: JsonValue = json::parse(&contents).unwrap();

    let base: HashMap<&str, &JsonValue> = parsed["base"].entries().collect();

    let width: u32 = base["width"].as_u32().unwrap();
    let height: u32 = base["height"].as_u32().unwrap();
    let pixels: Vec<u8> = base["pixels"]
        .members()
        .map(|x: &JsonValue| x.as_u8().unwrap())
        .collect();

    let palettes: JsonValue = parsed["palettes"].clone();

    let timeline: BTreeMap<u32, &str> = parsed["timeline"]
        .entries()
        .map(|(time, palette)| (time.parse::<u32>().unwrap(), palette.as_str().unwrap()))
        .collect();

    let mut last_time: &u32 = timeline.keys().last().unwrap();
    for (time, _) in timeline.iter() {
        if time > &seconds {
            break;
        }
        last_time = time;
    }

    let palette_raw: &JsonValue = &palettes[*timeline.get(last_time).unwrap()];

    let colors: Vec<[u8; 3]> = palette_raw["colors"]
        .members()
        .map(|x| {
            [
                x[0].as_u8().unwrap(),
                x[1].as_u8().unwrap(),
                x[2].as_u8().unwrap(),
            ]
        })
        .collect();

    let palette: Vec<Color> = colors
        .iter()
        .map(|x| Color::RGB(x[0], x[1], x[2]))
        .collect();

    let sdl_context: sdl2::Sdl = sdl2::init().unwrap();
    let video_subsystem: sdl2::VideoSubsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("Test", 800, 600)
        .position_centered()
        .borderless()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    let texture_creator: TextureCreator<WindowContext> = canvas.texture_creator();

    let mut texture = texture_creator
        .create_texture_target(None, width, height)
        .unwrap();

    canvas
        .with_texture_canvas(&mut texture, |texture_canvas| {
            texture_canvas.clear();

            for (i, &pixel) in pixels.iter().enumerate() {
                let i = i as u32;
                let color = palette[pixel as usize];
                let x = (i % width) as i32;
                let y = (i / height) as i32;

                texture_canvas.set_draw_color(color);
                texture_canvas
                    .draw_point(sdl2::rect::Point::new(x, y))
                    .unwrap();
            }
        })
        .unwrap();

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();

    canvas
        .copy(&texture, None, Rect::new(0, 0, 800, 600))
        .unwrap();

    let mut event_pump: sdl2::EventPump = sdl_context.event_pump().unwrap();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        canvas.present();
        std::thread::sleep(Duration::new(0, 1_000_000_000 / 15));
    }

    Ok(())
}

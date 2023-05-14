use chrono::{DateTime, Local, Timelike};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::prelude::*;

fn main() -> std::io::Result<()> {
    let now: DateTime<Local> = Local::now();
    let seconds = now.num_seconds_from_midnight();

    let file = File::open("scenes/April-Clear.json");
    let mut contents = String::new();
    file?.read_to_string(&mut contents)?;

    let parsed = json::parse(&contents).unwrap();

    let timeline: BTreeMap<u32, &str> = parsed["timeline"]
        .entries()
        .map(|(time, palette)| (time.parse::<u32>().unwrap(), palette.as_str().unwrap()))
        .collect();

    let mut last_time = timeline.keys().last().unwrap();
    let mut next_time = timeline.keys().next().unwrap();
    for (time, _) in timeline.iter() {
        last_time = next_time;
        next_time = time;
        if time > &seconds {
            break;
        }
    }
    println!("{} < {} < {}", last_time, seconds, next_time);

    Ok(())
}

extern crate ics_parser;

use std::{convert::TryInto, fs::File};
use std::{env, io::Read};

use ics_parser::{components::VCalendar, parser};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let path = args.get(1).map(|d| d as &str).unwrap_or("example.ics");

    let mut file = File::open(path).expect("valid file");
    let mut data = String::new();
    file.read_to_string(&mut data)?;

    let re = regex::Regex::new(r"\n[\t ]+").unwrap();
    let _data = re.replace_all(&data, "");

    let components = parser::Component::from_str_to_stream(&data)?;
    for comp in components {
        let calendar: VCalendar = comp.try_into()?;

        for (uid, event) in &calendar.events {
            if uid != "279a3ad2-7d2f-4f74-a2d1-d0bca5dc6227" {
                continue;
            }

            println!("Base event: {:#?}", event.base_event);

            // println!("{:?}:", event.base_event.summary);
            let times = event
                .recur_iter(&calendar)?
                .take(10)
                .map(|(d, _)| d)
                .collect::<Vec<_>>();
            // let times = event
            //     .base_event
            //     .recur_iter(&calendar)?
            //     .take(10)
            //     .collect::<Vec<_>>();
            println!("{}, {:?}: {:?}", uid, event.base_event.summary, times);
        }
    }

    Ok(())
}

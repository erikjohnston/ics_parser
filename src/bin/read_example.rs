use std::{
    convert::TryInto,
    io::{stdin, Read},
};

use ics_parser::{components::VCalendar, parser};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = stdin();
    let mut data = String::new();
    file.read_to_string(&mut data)?;

    // let re = regex::Regex::new(r"\n[\t ]+").unwrap();
    // let _data = re.replace_all(&data, "");

    let components = parser::Component::from_str_to_stream(&data)?;
    for comp in components {
        let calendar: VCalendar = comp.try_into()?;

        println!("Found {} events", calendar.events.len());
    }

    Ok(())
}

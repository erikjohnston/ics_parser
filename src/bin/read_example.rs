use std::{fs::File, io::Read};

use ics_parser::parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::open("~/temp/kegan.ics")?;
    let mut data = String::new();
    file.read_to_string(&mut data)?;

    // let re = regex::Regex::new(r"\n[\t ]+").unwrap();
    // let _data = re.replace_all(&data, "");

    let components = parser::Component::from_str_to_stream(&data)?;
    for comp in components {
        println!("{:#?}", comp);
        println!("{}", comp.as_string());
    }

    Ok(())
}

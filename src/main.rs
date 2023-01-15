use std::{{fs::File}, io::{Read}};

mod render;
mod agi_types;

use crate::agi_types::*;
use crate::render::*;

fn read_resources(directory_file_name : &str, volume_data : &Vec<Vec<u8>>) -> Result<Vec<Resource>, AgiError> {
    let mut directory_file_stream : Vec<u8> = Vec::new();
    let mut resource_entries : Vec<Resource> = Vec::new();

    let base_dir = "C:\\Program Files (x86)\\GOG Galaxy\\Games\\Kings Quest 3\\";
    let mut pic_dir_file = File::open(format!("{}{}", base_dir, directory_file_name))?;
    pic_dir_file.read_to_end(&mut directory_file_stream)?;

    for offset in 0..(directory_file_stream.len() / 3) {
        match Resource::new(AgiResourceType::Picture, &directory_file_stream, offset / 3, volume_data) {
            Ok(Some(val)) => resource_entries.push(val),
            Ok(None) => (),
            Err(err) => println!("Error parsing asset from {} at offset {}: {:?}", directory_file_name, offset, err)
        }
    }

    Ok(resource_entries)
}

fn main() -> Result<(), AgiError> {
    let vol_file_data : Vec<Vec<u8>> = (0..=3)
        .map(|i| {
            let mut volume_data : Vec<u8> = Vec::new();
            File::open(format!("C:\\Program Files (x86)\\GOG Galaxy\\Games\\Kings Quest 3\\VOL.{}", i)).unwrap().read_to_end(&mut volume_data).unwrap();
            volume_data
        })
        .collect();
    
    let resources = read_resources("PICDIR", &vol_file_data).unwrap();

    let pic_resource = PicResource::new(resources[6].get_raw_data())?;

    //for resource in resources {}

    render_window(pic_resource);

    Ok(())
}

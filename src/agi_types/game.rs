use crate::*;
use std::path::{Path};
use std::fs;
use std::io;

use super::pic::PicResource;

pub struct Game {
    pub dir_name : String,
    pub pic_resources : Vec<PicResource>,
    pub all_resources : Vec<Resource>
}

impl Game {
    pub fn new_from_dir(game_dir : &Path) -> Result<Self, AgiError> {

        let mut game_files = fs::read_dir(game_dir)?
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, io::Error>>()?;
        game_files.sort();

        let volume_data : Vec<Vec<u8>>= game_files.into_iter()
            .filter(|f| f.is_file() && f.file_name().unwrap_or_default().to_string_lossy().starts_with("VOL."))
            .map(|f| {
                let mut data : Vec<u8> = vec![];
                File::open(f).unwrap().read_to_end(&mut data).unwrap_or_default();
                data
            })
            .collect();

        let mut pic_data : Vec<u8> = vec![];
        File::open(game_dir.join("PICDIR"))?.read_to_end(&mut pic_data)?;

        let mut pic_resources : Vec<PicResource> = vec![];
        let mut all_resources : Vec<Resource> = vec![];
        for offset in (0..pic_data.len()).step_by(3) {
            match Resource::new(AgiResourceType::Picture, &pic_data, offset / 3, &volume_data) {
                Ok(Some(val)) => {
                    pic_resources.push(PicResource::new(val.get_raw_data()).unwrap());
                    all_resources.push(val);
                },
                Ok(None) => (),
                Err(err) => println!("Error parsing asset from {} at offset {}: {:?}", game_dir.join("PICDIR").to_str().unwrap_or("unknown"), offset, err)
            }
        }

        let game = Self {
            dir_name : game_dir.to_string_lossy().into_owned(),
            pic_resources,
            all_resources
        };

        Ok(game)
    }
}
use byteorder::*;
use crate::*;

#[cfg(test)]
use std::io::Write;

#[derive(Debug, PartialEq)]
#[allow(dead_code)]
pub enum AgiResourceType {
    Logic,
    Picture,
    View,
    Sound,
    Other
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct Resource {
    resource_type : AgiResourceType,
    resource_index : usize,
    vol_file: u8,
    vol_file_offset: usize,
    raw_data: Vec<u8>
}

/*
From: http://www.agidev.com/articles/agispec/agispecs-5.html

Each directory file is of the same format. They contain a finite number of three byte entries, no more than 256. The size will vary depending on the number of files of the type that the directory file is pointing to. Dividing the filesize by three gives the maximum file number of that type of data file. Each entry is of the following format:

    Byte 1           Byte 2           Byte 3
7 6 5 4 3 2 1 0  7 6 5 4 3 2 1 0  7 6 5 4 3 2 1 0
V V V V P P P P  P P P P P P P P  P P P P P P P P
where V = VOL number and P = position (offset into VOL file).

The entry number itself gives the number of the data file that it is pointing to. For example, if the following three byte entry is entry number 45 in the SOUND directory file,

12 3D FE
then sound.45 is located at position 0x23DFE in the vol.1 file. The first entry number is entry 0.

If the three bytes contain the value 0xFFFFFF, then the resource does not exist.
*/
impl Resource {
    pub fn get_raw_data(&self) -> &Vec<u8> {
        &self.raw_data
    }

    pub fn new(resource_type : AgiResourceType, directory_file_stream : &Vec<u8>, resource_index : usize, volume_files : &Vec<Vec<u8>> ) -> Result<Option<Self>, AgiError> {
        let stream_offset = resource_index * 3;

        if resource_index >= directory_file_stream.len() {
            Err(AgiError::Parse(format!("Stream was too short, asked for index {}, but only have {}", resource_index, directory_file_stream.len())))
        } else {
            let vol_file : u8 = directory_file_stream[stream_offset] >> 4;
            let vol_file_offset : usize = 
                (((directory_file_stream[stream_offset] as usize) & 0xFusize) << 16) |
                ((directory_file_stream[stream_offset+1] as usize) << 8) |
                (directory_file_stream[stream_offset+2] as usize);

            // Read the data from the volume file
            if vol_file == 0xF {
                Ok(None)
            } else if vol_file as usize >= volume_files.len()  && vol_file != 0xF {
               Err(AgiError::Parse(format!("Attempted to access invalid volume file index {}", vol_file)))
            } else {
                // Read the data from the volume file
                let my_vol_file_data = &volume_files[vol_file as usize];

                let signature : u16 = LittleEndian::read_u16(&my_vol_file_data[vol_file_offset..=vol_file_offset+1]);
                let resource_len : usize = LittleEndian::read_u16(&my_vol_file_data[vol_file_offset+3..=vol_file_offset+4]) as usize;

                if signature != 0x3412 {
                    return Err(AgiError::Parse(format!("Expected signature 0x3412, got {:#04x}", signature)))
                }

                Ok(Some(Self { resource_type, resource_index, vol_file, vol_file_offset, raw_data: my_vol_file_data[vol_file_offset + 5..vol_file_offset + 5 + resource_len].to_vec() }))
            }
        }
    }
}

#[cfg(test)]

#[test]
#[ignore]
fn generate_sample_files() {
    // This is a little piece of code to package up a few PIC resources for samples
    // It is used to generate the sample test set
    let paths_and_pics = vec![
        (Path::new("C:\\GOG Games\\Kings Quest\\"), vec![0,1,7,11,14,15,16,21,27,41,43,44,52,71,78]),
        (Path::new("C:\\GOG Games\\Kings Quest 2\\"), vec![0,1,2,3,6,8,9,10,19,20,43,58,59,63,67,69,96]),
        (Path::new("C:\\GOG Games\\Kings Quest 3\\"), vec![0,1,2,3,4,5,12,13,14,23,24,27,46,47,53,54,67,80,81]),
    ];

    let mut offset = 0usize;
    let mut volume_data : Vec<u8> = vec![];
    let mut picdir_data : Vec<u8> = vec![];

    let sig = [0x12u8, 0x34u8];

    for (path, pics) in paths_and_pics.iter() {
        if let Ok(game) = Game::new_from_dir(path) {
            for (pic_index, resource) in game.all_resources
                .iter()
                .filter(|r| r.resource_type == AgiResourceType::Picture)
                .enumerate() {
                
                if pics.contains(&pic_index) {
                    let index_data = [
                        (offset >> 16 & 0x0Fusize) as u8,
                        (offset >> 8 & 0xFFusize) as u8,
                        (offset & 0xFFusize) as u8
                    ];
                    picdir_data.extend(&index_data[..]);

                    let raw_data = resource.get_raw_data();
                    
                    // Write signature
                    volume_data.extend(sig);
                    
                    // Write volume index
                    volume_data.push(0);

                    // Write resource length
                    let mut len = [0u8; 2];
                    LittleEndian::write_u16(&mut len, raw_data.len() as u16);
                    volume_data.extend(len);

                    // Write data
                    volume_data.extend(raw_data);

                    offset += raw_data.len() + 5;
                }
            }
        }
    }

    // Write the samples to our files
    let mut volume_file = File::create("VOL.0").unwrap();
    volume_file.write_all(&volume_data).unwrap();

    let mut picdir_file = File::create("PICDIR").unwrap();
    picdir_file.write_all(&picdir_data).unwrap();
}
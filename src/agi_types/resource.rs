use byteorder::*;
use crate::*;

#[derive(Debug)]
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

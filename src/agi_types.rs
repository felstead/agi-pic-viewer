use std::collections::VecDeque;

use thiserror::Error;
use byteorder::*;

pub const VIEWPORT_WIDTH : usize = 160;
pub const VIEWPORT_HEIGHT : usize = 168;
pub const VIEWPORT_PIXELS : usize = VIEWPORT_WIDTH * VIEWPORT_HEIGHT;

#[derive(Error, Debug)]
pub enum AgiError {
    #[error("IO error")]
    IoError(#[from] std::io::Error),
    #[error("Parse error")]
    ParseError(String)
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

impl Resource {
    pub fn get_raw_data(&self) -> &Vec<u8> {
        &self.raw_data
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum AgiResourceType {
    Logic,
    Picture,
    View,
    Sound,
    Other
}

pub enum PictureBufferType {
    Picture,
    Priority
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
    pub fn new(resource_type : AgiResourceType, directory_file_stream : &Vec<u8>, resource_index : usize, volume_files : &Vec<Vec<u8>> ) -> Result<Option<Self>, AgiError> {
        let stream_offset = resource_index * 3;

        if resource_index >= directory_file_stream.len() {
            Err(AgiError::ParseError(format!("Stream was too short, asked for index {}, but only have {}", resource_index, directory_file_stream.len())))
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
               Err(AgiError::ParseError(format!("Attempted to access invalid volume file index {}", vol_file)))
            } else {
                // Read the data from the volume file
                let my_vol_file_data = &volume_files[vol_file as usize];

                let signature : u16 = LittleEndian::read_u16(&my_vol_file_data[vol_file_offset..=vol_file_offset+1]);
                let resource_len : usize = LittleEndian::read_u16(&my_vol_file_data[vol_file_offset+3..=vol_file_offset+4]) as usize;

                if signature != 0x3412 {
                    return Err(AgiError::ParseError(format!("Expected signature 0x3412, got {:#04x}", signature)))
                }

                Ok(Some(Self { resource_type, resource_index, vol_file, vol_file_offset, raw_data: my_vol_file_data[vol_file_offset + 5..vol_file_offset + 5 + resource_len].to_vec() }))
            }
        }
    }
}


pub struct PicResource {
    pic_buffer : Vec<FrameBufferPixel>,
    pri_buffer : Vec<FrameBufferPixel>,
    instructions : Vec<PicRenderInstruction>
}

impl Default for PicResource {
    fn default() -> Self {
        PicResource { 
            pic_buffer:  vec![FrameBufferPixel::default_picture_buffer(); VIEWPORT_PIXELS],
            pri_buffer: vec![FrameBufferPixel::default_priority_buffer(); VIEWPORT_PIXELS],
            instructions: vec![]
        }
    }
}

impl PicResource {
    pub fn new(raw_data : &Vec<u8>) -> Result<Self, AgiError> {
        // Read the instructions
        let mut offset = 0usize;

        let mut resource = PicResource::default();

        while offset < raw_data.len() {
            let (instruction, next_offset) = PicRenderInstruction::create_from_vec(raw_data, offset);
            resource.instructions.push(instruction);
            offset = next_offset;
        }

        resource.render(resource.instructions.len() - 1);

        Ok(resource)
    }

    pub fn get_instructions(&self) -> &Vec<PicRenderInstruction> {
        &self.instructions
    }

    pub fn get_buffer(&self, buffer_type : &PictureBufferType) -> &Vec<FrameBufferPixel> {
        match buffer_type {
            PictureBufferType::Picture => &self.pic_buffer,
            PictureBufferType::Priority => &self.pri_buffer
        }
    }

    pub fn render(&mut self, max_index: usize) {

        let mut pic_color : Option<u8> = None;
        let mut pri_color : Option<u8> = None;

        // Clear buffers
        self.pic_buffer = vec![FrameBufferPixel::default_picture_buffer(); VIEWPORT_PIXELS];
        self.pri_buffer = vec![FrameBufferPixel::default_priority_buffer(); VIEWPORT_PIXELS];

        for instruction_index in 0..=max_index {
            let inst = self.instructions[instruction_index].clone(); // Copy

            match inst {
                PicRenderInstruction::SetPicColorAndEnablePicDraw(args) => pic_color = Some(args[0]),
                PicRenderInstruction::DisablePicDraw => pic_color = None,
                PicRenderInstruction::SetPriColorAndEnablePriDraw(args) => pri_color = Some(args[0]),
                PicRenderInstruction::DisablePriDraw => pri_color = None,
                PicRenderInstruction::DrawYCorner(args) => self.draw_corner_line(&args, false, pic_color, pri_color, instruction_index),
                PicRenderInstruction::DrawXCorner(args) => self.draw_corner_line(&args, true, pic_color, pri_color, instruction_index),
                PicRenderInstruction::AbsLine(args) => self.draw_abs_line(&args, pic_color, pri_color, instruction_index),
                PicRenderInstruction::RelLine(args) => self.draw_rel_line(&args, pic_color, pri_color, instruction_index),
                PicRenderInstruction::Fill(args) => self.fill(&args, pic_color, pri_color, instruction_index),
                PicRenderInstruction::SetPenSizeAndStyle(args) => (),
                PicRenderInstruction::PlotWithPen(args) => (),
                PicRenderInstruction::EndInstruction => (),
                PicRenderInstruction::Unknown(code) => ()
            }
        }
    }

    // This function and the next are ported from here: http://www.agidev.com/articles/agispec/agispecs-7.html#ss7.1
    // Basically the arguments are pairs of coordinates to draw lines between
    fn draw_abs_line(&mut self, args : &Vec<u8>, pic_color : Option<u8>, pri_color : Option<u8>, instruction_index : usize) {
        let (mut x1, mut y1) = (args[0] as usize, args[1] as usize);

        if args.len() == 2 {
            // Just draw a single pixel
            self.set_pixel(x1, y1, pic_color, pri_color, instruction_index);
        } else {
            for i in (2..args.len()).step_by(2) {
                let (x2, y2) = (args[i] as usize, args[i+1] as usize);
    
                let (height, width) = (y2 as i32 - y1 as i32, x2 as i32 - x1 as i32);
    
                let mut add_x = if height == 0 { 0.0 } else { width as f32 / (height as f32).abs() };
                let mut add_y = if width == 0 { 0.0 } else { height as f32 / (width as f32).abs() };
    
                let (mut x, mut y) = (x1 as f32, y1 as f32);
                if width.abs() > height.abs() {
                    add_x = width.signum() as f32;
                    
                    while (x - x2 as f32).abs() > f32::EPSILON {
                        self.set_pixel(
                            Self::sierra_round(x, add_x),
                            Self::sierra_round(y, add_y),
                            pic_color,
                            pri_color,
                            instruction_index);
    
                        x += add_x;
                        y += add_y;
                    }
                    self.set_pixel(x2, y2, pic_color, pri_color, instruction_index);
                } else {
                    add_y = height.signum() as f32;
    
                    while (y - y2 as f32).abs() > f32::EPSILON {
                        self.set_pixel(
                            Self::sierra_round(x, add_x),
                            Self::sierra_round(y, add_y),
                            pic_color,
                            pri_color,
                            instruction_index);
    
                        x += add_x;
                        y += add_y;
                    }
                    self.set_pixel(x2, y2, pic_color, pri_color, instruction_index);
                }
    
                (x1, y1) = (x2, y2)
            }
        }
    }

    fn draw_rel_line(&mut self, args : &Vec<u8>, pic_color : Option<u8>, pri_color : Option<u8>, instruction_index : usize) {
        // Convert the relative arguments to absolute
        let mut abs_lines = vec![args[0], args[1]];
        let (mut x, mut y) = (args[0], args[1]);
        for i in 2..args.len() {
            let arg = args[i];
            let sign_x = if 0x80 & arg > 0 { -1i8 } else { 1i8 };
            let sign_y = if 0x08 & arg > 0 { -1i8 } else { 1i8 };

            let disp_x1 = (arg & 0x70) >> 4;
            let disp_x1i = disp_x1 as i8;
            let disp_y1 = arg & 0x07;

            let disp_x = sign_x * disp_x1i;
            let disp_y = sign_y * (arg & 0x07) as i8;

            let (x1, y1) = ((x as i16 + disp_x as i16) as u8, (y as i16 + disp_y as i16) as u8);
            abs_lines.push(x1);
            abs_lines.push(y1);

            x = x1;
            y = y1;
        }
        self.draw_abs_line(&abs_lines, pic_color, pri_color, instruction_index);
    }

    fn draw_corner_line(&mut self, args : &Vec<u8>, start_on_x : bool, pic_color : Option<u8>, pri_color : Option<u8>, instruction_index : usize) {
        
        let mut abs_lines = vec![args[0], args[1]];
        let (mut x, mut y) = (args[0], args[1]);
        let mut direction_is_x = start_on_x;

        for i in 2..args.len() {
            let arg = args[i];

            if direction_is_x {
                x = arg;
            } else {
                y = arg;
            }

            abs_lines.push(x);
            abs_lines.push(y);

            direction_is_x = !direction_is_x;
        }

        self.draw_abs_line(&abs_lines, pic_color, pri_color, instruction_index);
    }

    fn fill(&mut self, args : &Vec<u8>, pic_color : Option<u8>, pri_color : Option<u8>, instruction_index : usize) {
        for i in (0..args.len()).step_by(2) {
            
            if let Some(color) = pic_color {
                // Do our fill
                let mut fill_queue = VecDeque::from([(args[i] as usize, args[i+1] as usize)]);

                while !fill_queue.is_empty() {
                    let (cur_x, cur_y) = fill_queue.pop_front().unwrap();

                    let curr_buffer_index = cur_y * 160 + cur_x;

                    if self.pic_buffer[curr_buffer_index].color == 0x0F {
                        // Fill this and add our surroundings
                        self.pic_buffer[curr_buffer_index].color = color;
                        self.pic_buffer[curr_buffer_index].instruction_indexes.push(instruction_index);

                        if cur_x < VIEWPORT_WIDTH - 1 { 
                            fill_queue.push_back((cur_x+1, cur_y));
                        }
                        if cur_x > 0 { 
                            fill_queue.push_back((cur_x-1, cur_y));
                        }

                        if cur_y < VIEWPORT_HEIGHT - 1 {
                            fill_queue.push_back((cur_x, cur_y+1));
                        }

                        if cur_y > 0 {
                            fill_queue.push_back((cur_x, cur_y-1));
                        }
                    }
                }
            }
        }
    }

    fn sierra_round(num : f32, dir : f32) -> usize {
        if dir < 0.0 {
            if num - num.floor() <= 0.501 { 
                num.floor() as usize
            } else { 
                num.ceil() as usize 
            }
        } else {
            if num - num.floor() < 0.499 {
                num.floor() as usize
            } else {
                num.ceil() as usize
            }
        }

    }

    fn set_pixel(&mut self, x : usize, y : usize, pic_color : Option<u8>, pri_color : Option<u8>, instruction_index : usize) {
        let buffer_index = y * 160 + x;

        if let Some(color) = pic_color {
            self.pic_buffer[buffer_index].color = color;
            self.pic_buffer[buffer_index].instruction_indexes.push(instruction_index);
        }

        if let Some(color) = pri_color {
            self.pri_buffer[buffer_index].color = color;
            self.pri_buffer[buffer_index].instruction_indexes.push(instruction_index);
        }
    }
}

#[derive(Clone)]
pub struct FrameBufferPixel {
    pub color : u8,
    pub instruction_indexes : Vec<usize>
}

impl FrameBufferPixel {
    pub fn default_picture_buffer() -> Self {
        FrameBufferPixel { color: 0x0F, instruction_indexes: vec![] }
    }

    pub fn default_priority_buffer() -> Self {
        FrameBufferPixel { color: 0x04, instruction_indexes: vec![] }
    }
}

#[derive(Debug, Clone)]
pub enum PicRenderInstruction {
    SetPicColorAndEnablePicDraw(Vec<u8>),
    DisablePicDraw,
    SetPriColorAndEnablePriDraw(Vec<u8>),
    DisablePriDraw,
    DrawYCorner(Vec<u8>),
    DrawXCorner(Vec<u8>),
    AbsLine(Vec<u8>),
    RelLine(Vec<u8>),
    Fill(Vec<u8>),
    SetPenSizeAndStyle(Vec<u8>),
    PlotWithPen(Vec<u8>),
    EndInstruction,
    Unknown(u8)
}

impl PicRenderInstruction {
    pub fn create_from_vec(raw_data : &Vec<u8>, offset : usize) -> (PicRenderInstruction, usize) {
        let instruction = raw_data[offset];

        // Extract the arguments
        let mut current_offset = offset + 1;
        while current_offset < raw_data.len() && raw_data[current_offset] & 0xF0 != 0xF0 {
            current_offset += 1;
        }
        
        let arguments = raw_data[offset+1..current_offset].to_vec();

        let decoded_instruction = match instruction {
            0xF0 => Self::SetPicColorAndEnablePicDraw(arguments),
            0xF1 => Self::DisablePicDraw,
            0xF2 => Self::SetPriColorAndEnablePriDraw(arguments),
            0xF3 => Self::DisablePriDraw,
            0xF4 => Self::DrawYCorner(arguments),
            0xF5 => Self::DrawXCorner(arguments),
            0xF6 => Self::AbsLine(arguments),
            0xF7 => Self::RelLine(arguments),
            0xF8 => Self::Fill(arguments),
            0xF9 => Self::SetPenSizeAndStyle(arguments),
            0xFA => Self::PlotWithPen(arguments),
            0xFF => Self::EndInstruction,
            _ => Self::Unknown(instruction)
        };

        (decoded_instruction, current_offset)
    }
}

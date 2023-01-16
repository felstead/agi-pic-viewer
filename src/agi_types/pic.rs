
use std::collections::VecDeque;
use egui::ecolor::*;

use crate::*;

pub enum PictureBufferType {
    Picture,
    Priority
}



pub struct PicResource {
    pic_buffer : Vec<FrameBufferPixel>,
    pri_buffer : Vec<FrameBufferPixel>,
    instructions : Vec<PicRenderInstruction>,
    pic_raster_data : Vec<Color32>,
    pri_raster_data : Vec<Color32>,
    rasterize_on_render : bool
}

impl PicResource {
    pub fn new(raw_data : &Vec<u8>) -> Result<Self, AgiError> {
        // Read the instructions
        let mut offset = 0usize;

        let mut resource = PicResource { 
            pic_buffer:  vec![],
            pri_buffer: vec![],
            instructions: vec![],
            rasterize_on_render: true,
            pic_raster_data: vec![],
            pri_raster_data: vec![]
        };

        while offset < raw_data.len() {
            let (instruction, next_offset) = PicRenderInstruction::create_from_vec(raw_data, offset);
            resource.instructions.push(instruction);
            offset = next_offset;
        }

        resource.render(resource.instructions.len() - 1, true);

        Ok(resource)
    }

    pub fn get_color(agi_color : u8) -> Color32 {
        // From here: https://moddingwiki.shikadi.net/wiki/EGA_Palette
        match agi_color {
            0x00 => Color32::from_rgb(0x00,0x00,0x00), // black
            0x01 => Color32::from_rgb(0x00,0x00,0xAA), // blue
            0x02 => Color32::from_rgb(0x00,0xAA,0x00), // green
            0x03 => Color32::from_rgb(0x00,0xAA,0xAA), // cyan
            0x04 => Color32::from_rgb(0xAA,0x00,0x00), // red
            0x05 => Color32::from_rgb(0xAA,0x00,0xAA), // magenta
            0x06 => Color32::from_rgb(0xAA,0x55,0x00), // brown
            0x07 => Color32::from_rgb(0xAA,0xAA,0xAA), // light gray
            0x08 => Color32::from_rgb(0x55,0x55,0x55), // dark gray
            0x09 => Color32::from_rgb(0x55,0x55,0xFF), // light blue
            0x0A => Color32::from_rgb(0x55,0xFF,0x55), // light green
            0x0B => Color32::from_rgb(0x55,0xFF,0xFF), // light cyan
            0x0C => Color32::from_rgb(0xFF,0x55,0x55), // light red
            0x0D => Color32::from_rgb(0xFF,0x55,0xFF), // light magenta
            0x0E => Color32::from_rgb(0xFF,0xFF,0x55), // yellow
            0x0F => Color32::from_rgb(0xFF,0xFF,0xFF), // white
            _ => Color32::from_rgb(0xFF,0x00,0xFF),
        }
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

    pub fn get_raster_data(&self, buffer_type : &PictureBufferType) -> &Vec<Color32> {
        match buffer_type {
            PictureBufferType::Picture => &self.pic_raster_data,
            PictureBufferType::Priority => &self.pri_raster_data
        }
    }

    pub fn render(&mut self, max_index: usize, rasterize : bool) {

        let mut pic_color : Option<u8> = None;
        let mut pri_color : Option<u8> = None;

        // Clear buffers
        self.pic_buffer = vec![FrameBufferPixel::default_picture_buffer(); VIEWPORT_PIXELS];
        self.pri_buffer = vec![FrameBufferPixel::default_priority_buffer(); VIEWPORT_PIXELS];

        self.rasterize_on_render = rasterize;
        if self.rasterize_on_render {
            self.pic_raster_data = vec![Self::get_color(0x0F); VIEWPORT_PIXELS];
            self.pri_raster_data = vec![Self::get_color(0x04); VIEWPORT_PIXELS];
        }

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
                PicRenderInstruction::SetPenSizeAndStyle(_args) => (),
                PicRenderInstruction::PlotWithPen(_args) => (),
                PicRenderInstruction::EndInstruction => (),
                PicRenderInstruction::Unknown(_code) => ()
            }
        }
    }

    // This function and the next are ported from here: http://www.agidev.com/articles/agispec/agispecs-7.html#ss7.1
    // Basically the arguments are pairs of coordinates to draw lines between
    fn draw_abs_line(&mut self, args : &Vec<u8>, pic_color : Option<u8>, pri_color : Option<u8>, instruction_index : usize) {
        if args.is_empty() || args.len() % 2 != 0 {
            // TODO: Log error
            return;
        }

        let (mut x1, mut y1) = (args[0] as usize, args[1] as usize);

        if args.len() == 2 {
            // Just draw a single pixel
            self.set_both_buffer_pixel(x1, y1, pic_color, pri_color, instruction_index);
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
                        self.set_both_buffer_pixel(
                            Self::sierra_round(x, add_x),
                            Self::sierra_round(y, add_y),
                            pic_color,
                            pri_color,
                            instruction_index);
    
                        x += add_x;
                        y += add_y;
                    }
                    self.set_both_buffer_pixel(x2, y2, pic_color, pri_color, instruction_index);
                } else {
                    add_y = height.signum() as f32;
    
                    while (y - y2 as f32).abs() > f32::EPSILON {
                        self.set_both_buffer_pixel(
                            Self::sierra_round(x, add_x),
                            Self::sierra_round(y, add_y),
                            pic_color,
                            pri_color,
                            instruction_index);
    
                        x += add_x;
                        y += add_y;
                    }
                    self.set_both_buffer_pixel(x2, y2, pic_color, pri_color, instruction_index);
                }
    
                (x1, y1) = (x2, y2)
            }
        }
    }

    fn draw_rel_line(&mut self, args : &Vec<u8>, pic_color : Option<u8>, pri_color : Option<u8>, instruction_index : usize) {

        if args.len() < 2 {
            // TODO: Log error
            return;
        }

        // Convert the relative arguments to absolute
        let mut abs_lines = vec![args[0], args[1]];
        let (mut x, mut y) = (args[0], args[1]);
        for arg in args.iter().skip(2) {
            let sign_x = if 0x80 & arg > 0 { -1i8 } else { 1i8 };
            let sign_y = if 0x08 & arg > 0 { -1i8 } else { 1i8 };

            let disp_x = sign_x * ((arg & 0x70) >> 4) as i8;
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
        
        if args.len() < 2 {
            // TODO: Log error
            return;
        }

        let mut abs_lines = vec![args[0], args[1]];
        let (mut x, mut y) = (args[0], args[1]);
        let mut direction_is_x = start_on_x;

        for arg in args.iter().skip(2) {
            if direction_is_x {
                x = *arg;
            } else {
                y = *arg;
            }

            abs_lines.push(x);
            abs_lines.push(y);

            direction_is_x = !direction_is_x;
        }

        self.draw_abs_line(&abs_lines, pic_color, pri_color, instruction_index);
    }

    fn fill(&mut self, args : &Vec<u8>, pic_color : Option<u8>, pri_color : Option<u8>, instruction_index : usize) {
        if args.len() % 2 != 0 {
            // TODO: Log error
            return;
        }

        for i in (0..args.len()).step_by(2) {

            if let Some(color) = pic_color {
                self.fill_specific_buffer(args[i] as usize, args[i+1] as usize, color, &PictureBufferType::Picture, instruction_index);
            }

            if let Some(color) = pri_color {
                self.fill_specific_buffer(args[i] as usize, args[i+1] as usize, color, &PictureBufferType::Priority, instruction_index);
            }
/*
                // Do our fill
                let mut fill_queue = VecDeque::from([(args[i] as usize, args[i+1] as usize)]);

                while !fill_queue.is_empty() {
                    let (cur_x, cur_y) = fill_queue.pop_front().unwrap();

                    let curr_buffer_index = cur_y * 160 + cur_x;

                    if self.pic_buffer[curr_buffer_index].color == 0x0F {
                        // Fill this and add our surroundings
                        self.set_pixel(cur_x, cur_y, Some(color), None, instruction_index);

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
            }*/
        }
    }

    fn fill_specific_buffer(&mut self, x : usize, y : usize, color : u8, buffer_type : &PictureBufferType, instruction_index : usize) {

        let default_color = match &buffer_type {
            PictureBufferType::Picture => 0x0Fu8,
            PictureBufferType::Priority => 0x04u8
        };

        if color == default_color {
            // Filling with the default color is verboten
            return;
        }

        // Do our fill
        let mut fill_queue = VecDeque::from([(x, y)]);

        while !fill_queue.is_empty() {
            let (cur_x, cur_y) = fill_queue.pop_front().unwrap();
            if self.get_single_buffer_pixel_color(cur_x, cur_y, buffer_type) == default_color {
                // Fill this and add our surroundings
                self.set_single_buffer_pixel(cur_x, cur_y, color, buffer_type, instruction_index);

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

    fn sierra_round(num : f32, dir : f32) -> usize {
        if dir < 0.0 {
            if num - num.floor() <= 0.501 { 
                num.floor() as usize
            } else { 
                num.ceil() as usize 
            }
        } else if num - num.floor() < 0.499 {
            num.floor() as usize
        } else {
            num.ceil() as usize
        }
    }

    fn set_both_buffer_pixel(&mut self, x : usize, y : usize, pic_color : Option<u8>, pri_color : Option<u8>, instruction_index : usize) {
        if let Some(color) = pic_color {
            self.set_single_buffer_pixel(x, y, color, &PictureBufferType::Picture, instruction_index);
        }

        if let Some(color) = pri_color {
            self.set_single_buffer_pixel(x, y, color, &PictureBufferType::Priority, instruction_index);
        }
    }

    fn get_single_buffer_pixel_color(&self, x : usize, y : usize, buffer_type : &PictureBufferType) -> u8 {
        let buffer_index = y * VIEWPORT_WIDTH + x;

        match buffer_type {
            PictureBufferType::Picture => self.pic_buffer[buffer_index].color,
            PictureBufferType::Priority => self.pri_buffer[buffer_index].color
        }
    }

    fn set_single_buffer_pixel(&mut self, x : usize, y : usize, color : u8, buffer_type : &PictureBufferType, instruction_index : usize) {
        let (buffer, raster_buffer) = match &buffer_type {
            PictureBufferType::Picture => (&mut self.pic_buffer, &mut self.pic_raster_data),
            PictureBufferType::Priority => (&mut self.pri_buffer, &mut self.pri_raster_data)
        };

        let buffer_index = y * VIEWPORT_WIDTH + x;
        buffer[buffer_index].color = color;
        buffer[buffer_index].instruction_indexes.push(instruction_index);

        if self.rasterize_on_render {
            raster_buffer[buffer_index] = Self::get_color(color);
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

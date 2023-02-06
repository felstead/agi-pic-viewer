use std::collections::VecDeque;
use egui::*;
use crate::*;

#[derive(Default, Debug, Copy, Clone)]
pub struct InstructionIndex {
    #[allow(dead_code)]
    base_index : u16,
    #[allow(dead_code)]
    sub_index : u16
}

impl InstructionIndex {
    pub fn new_sub(base_index : usize, sub_index : usize) -> Self {
        Self {
            base_index : base_index as u16,
            sub_index : sub_index as u16
        }
    }
}

pub struct PixelBuffer {
    pixels : Box<[Color32 ; VIEWPORT_PIXELS]>,
    instruction_indexes : Box<[Option<InstructionIndex> ; VIEWPORT_PIXELS]>
}

impl PixelBuffer {
    pub fn new(default_color : Color32) -> Self {
        Self {
            pixels : Box::new([default_color ; VIEWPORT_PIXELS]),
            instruction_indexes : Box::new([Some(InstructionIndex::default()) ; VIEWPORT_PIXELS]),
        }
    }

    pub fn set_pixel(&mut self, x : usize, y : usize, color : Option<u8>, instruction_index : InstructionIndex) -> Result<(), AgiError> {
        if let Some(color) = color {
            let index = y * VIEWPORT_WIDTH + x;
            if index >= VIEWPORT_PIXELS {
                return Err(AgiError::Render(format!("Pixel location ({x},{y}) out of range!")));
            } else {
                self.pixels[index] = get_color(color);
                self.instruction_indexes[index] = Some(instruction_index);
            }
        }

        Ok(())
    }

    pub fn get_pixels(&self) -> &[Color32] {
        self.pixels.as_ref()
    }

    pub fn get_pixels_vec(&self) -> Vec<Color32> {
        self.pixels.to_vec()
    }

    pub fn get_pixel(&self, x : usize, y : usize) -> Result<Color32, AgiError> {
        let index = y * VIEWPORT_WIDTH + x;
        if index >= VIEWPORT_PIXELS {
            Err(AgiError::Render(format!("Pixel location ({x},{y}) out of range!")))
        } else {
            Ok(self.pixels[index])
        }
    }
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

pub fn render_pixels(instructions : &[DerivedPicRenderInstruction], pic_buffer : &mut Option<&mut PixelBuffer>, pri_buffer : &mut Option<&mut PixelBuffer>) -> Result<(), AgiError> {
    if let Some(pic_buffer) = pic_buffer {
        pic_buffer.pixels.iter_mut().for_each(|c| *c = get_color(PIC_BUFFER_BASE_COLOR));
    }

    if let Some(pri_buffer) = pri_buffer {
        pri_buffer.pixels.iter_mut().for_each(|c| *c = get_color(PRI_BUFFER_BASE_COLOR));
    }

    let mut pic_color = Some(PIC_BUFFER_BASE_COLOR);
    let mut pri_color = Some(PRI_BUFFER_BASE_COLOR);

    for (instruction_index, instruction) in instructions.iter().enumerate() {
        match instruction {
            DerivedPicRenderInstruction::SetColor(_, buffer_type, color) => {
                match buffer_type {
                    PictureBufferType::Picture => pic_color = *color,
                    PictureBufferType::Priority => pri_color = *color
                }
            },
            DerivedPicRenderInstruction::DrawLines(_, lines) => {
                draw_lines(lines, pic_buffer, pic_color, pri_buffer, pri_color, instruction_index)?
            },
            DerivedPicRenderInstruction::Fill(_, points) => {
                fill(points, pic_buffer, pic_color, pri_buffer, pri_color, instruction_index)?
            },
            DerivedPicRenderInstruction::Unimplemented(_orignal_inst) => {
                // TODO: Log?
            }
        }
    }

    Ok(())
}

fn draw_lines(lines : &Vec<PosU8>, pic_buffer : &mut Option<&mut PixelBuffer>, pic_color : Option<u8>, pri_buffer : &mut Option<&mut PixelBuffer>, pri_color : Option<u8>, instruction_index : usize) -> Result<(), AgiError> {

    if lines.len() > 0 {
        let (mut x1, mut y1) = (lines[0].x as usize, lines[0].y as usize);

        let mut set_buffers_pixels = |x : usize, y : usize, sub_index : usize| -> Result<(), AgiError> {
    
            if let Some(pic_buffer) = pic_buffer {
                pic_buffer.set_pixel(x, y, pic_color, InstructionIndex::new_sub(instruction_index, sub_index))?;
            }
    
            if let Some(pri_buffer) = pri_buffer {
                pri_buffer.set_pixel(x, y, pri_color, InstructionIndex::new_sub(instruction_index, sub_index))?;
            }
    
            Ok(())
        };
    
        if lines.len() == 1 {
            // Just draw a single pixel
            set_buffers_pixels(x1, y1, 0)?;
        } else {
            for (line_index, line) in lines.iter().enumerate().skip(1) {
                let (x2, y2) = (line.x as usize, line.y as usize);
    
                let (height, width) = (y2 as i32 - y1 as i32, x2 as i32 - x1 as i32);
    
                let mut add_x = if height == 0 { 0.0 } else { width as f32 / (height as f32).abs() };
                let mut add_y = if width == 0 { 0.0 } else { height as f32 / (width as f32).abs() };
    
                let (mut x, mut y) = (x1 as f32, y1 as f32);
                if width.abs() > height.abs() {
                    add_x = width.signum() as f32;
                    
                    while (x - x2 as f32).abs() > f32::EPSILON {
                        set_buffers_pixels(
                            sierra_round(x, add_x),
                            sierra_round(y, add_y),
                            line_index)?;
    
                        x += add_x;
                        y += add_y;
                    }
    
                    set_buffers_pixels(x2, y2, line_index)?;
                } else {
                    add_y = height.signum() as f32;
    
                    while (y - y2 as f32).abs() > f32::EPSILON {
                        set_buffers_pixels(
                            sierra_round(x, add_x),
                            sierra_round(y, add_y),
                            line_index)?;
    
                        x += add_x;
                        y += add_y;
                    }
                    set_buffers_pixels(x2, y2, line_index)?;
                }
    
                (x1, y1) = (x2, y2)
            }
        }
    }

    Ok(())
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

fn fill(points : &[PosU8], pic_buffer : &mut Option<&mut PixelBuffer>, pic_color : Option<u8>, pri_buffer : &mut Option<&mut PixelBuffer>, pri_color : Option<u8>, instruction_index : usize) -> Result<(), AgiError> {

    for (sub_index, point) in points.iter().enumerate() {
        if let (Some(pic_buffer), Some(pic_color)) = (&mut *pic_buffer, pic_color) {
            fill_specific_buffer(*point, pic_color, &PictureBufferType::Picture, pic_buffer, instruction_index, sub_index)?;
        }

        if let (Some(pri_buffer), Some(pri_color)) = (&mut *pri_buffer, pri_color) {
            fill_specific_buffer(*point, pri_color, &PictureBufferType::Priority, pri_buffer, instruction_index, sub_index)?;
        }
    }

    Ok(())
}

fn fill_specific_buffer(point : PosU8, color : u8, buffer_type : &PictureBufferType, buffer : &mut PixelBuffer, instruction_index : usize, sub_index : usize) -> Result<(), AgiError> {

    let default_color = match &buffer_type {
        PictureBufferType::Picture => 0x0Fu8,
        PictureBufferType::Priority => 0x04u8
    };

    if color == default_color {
        // Filling with the default color is a no-op 
        return Ok(());
    }

    // Do our fill
    let mut fill_queue = VecDeque::from([(point.x as usize, point.y as usize)]);

    while !fill_queue.is_empty() {
        let (cur_x, cur_y) = fill_queue.pop_front().unwrap();


        if buffer.get_pixel(cur_x, cur_y).unwrap() == get_color(default_color) {
            // Fill this and add our surroundings
            buffer.set_pixel(cur_x, cur_y, Some(color), InstructionIndex::new_sub(instruction_index, sub_index))?;

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

    Ok(())
}
use std::collections::{VecDeque};
use egui::*;
use crate::*;

#[derive(Default)]
pub struct RenderOptions {
    pub render_only_selected_instruction : bool,
    pub show_fill_outlines : bool,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct InstructionIndex {
    #[allow(dead_code)]
    base_index : u16,
    #[allow(dead_code)]
    sub_index : u16
}

// Internally used to track the edges of a fill
#[derive(PartialEq, Eq)]
enum FillEdge {
    Line(InstructionIndex),
    TopBorder,
    RightBorder,
    BottomBorder,
    LeftBorder
}

impl FillEdge {

    const MAX_X : f32 = (VIEWPORT_WIDTH-1) as f32;
    const MAX_Y : f32 = (VIEWPORT_HEIGHT-1) as f32;
    const TOP_LEFT : Pos2 = pos2(0f32, 0f32);
    const TOP_RIGHT : Pos2 = pos2(Self::MAX_X, 0f32);
    const BOTTOM_RIGHT : Pos2 = pos2(Self::MAX_X, Self::MAX_Y);
    const BOTTOM_LEFT : Pos2 = pos2(0f32, Self::MAX_Y);

    const TOP_BORDER : [Pos2; 2] = [Self::TOP_LEFT, Self::TOP_RIGHT];
    const RIGHT_BORDER : [Pos2; 2] = [Self::TOP_RIGHT, Self::BOTTOM_RIGHT];
    const BOTTOM_BORDER : [Pos2; 2] = [Self::BOTTOM_RIGHT, Self::BOTTOM_LEFT];
    const LEFT_BORDER : [Pos2; 2] = [Self::BOTTOM_LEFT, Self::TOP_LEFT];

    pub fn to_line(&self, instructions : &[DerivedPicRenderInstruction]) -> Result<[Pos2; 2], AgiError> {
        match self {
            Self::Line(inst) => {
                if (inst.base_index as usize) < instructions.len() {
                    let bi = inst.base_index as usize;
                    match &instructions[bi] {
                        DerivedPicRenderInstruction::DrawLines(_, points) => {
                            let si = inst.sub_index as usize;
                            if points.len() == 1 && si == 0 {
                                // Special case for single pixel lines
                                Ok([points[si].to_pos2(), points[si].to_pos2()])
                            } else if (si) < (points.len() - 1) {
                                Ok([points[si].to_pos2(), points[si+1].to_pos2()])
                            } else {
                                Err(AgiError::Render(format!("Sub index was out of bounds")))
                            }
                        },
                        _ => Err(AgiError::Render(format!("Fill instruction adjacency touched a non line instruction")))

                    }
                    
                } else {
                    Err(AgiError::Render(format!("Instruction index was out of range!")))
                }
            },
            Self::TopBorder => Ok(Self::TOP_BORDER),
            Self::RightBorder => Ok(Self::RIGHT_BORDER),
            Self::BottomBorder => Ok(Self::BOTTOM_BORDER),
            Self::LeftBorder => Ok(Self::LEFT_BORDER)
        }
    }
}

/* 
#[derive(Debug, PartialEq, Eq, Hash)]
struct LineU8 {
    start : [u8; 2],
    end : [u8 ; 2]
}

fn line_u8(start_x : u8, start_y : u8, end_x : u8, end_y : u8) -> LineU8 {
    LineU8 {
        start: [start_x, start_y],
        end: [end_x, end_y]
    }
}
*/

impl InstructionIndex {
    pub fn new_sub(base_index : usize, sub_index : usize) -> Self {
        Self {
            base_index : base_index as u16,
            sub_index : sub_index as u16
        }
    }
}

pub struct VectorPath {
    pub points : Vec<Pos2>,
    pub color : Color32
}

impl VectorPath {
    pub fn from_point_list(points : &Vec<PosU8>, color : u8) -> VectorPath {
        VectorPath { 
            points: points.iter().map(|p| pos2(p.x as f32, p.y as f32)).collect::<Vec<Pos2>>(), 
            color: get_color(color)
        }
    }

    pub fn from_line(points : [Pos2; 2], color : u8) -> VectorPath {
        VectorPath {
            points: points.to_vec(),
            color: get_color(color)
        }
    }
}

pub struct VectorFill {
    vertices : Vec<Pos2>,
    triangles : Vec<usize> // Indexed into vertices
}

fn vector_fill_from_fill_edges(edges_list : Vec<FillEdge>) -> Result<VectorFill, AgiError> {
    let mut fill = VectorFill {
        vertices : vec![],
        triangles : vec![]
    };

    // Convert the fill edges to vertices

    Ok(fill)
}


pub struct ShapeBuffer {
    paths : Vec<VectorPath>,
    _fills : Vec<VectorFill>
}

impl ShapeBuffer {
    pub fn new() -> Self {
        Self {
            paths : vec![],
            _fills : vec![]
        }
    }

    pub fn clear(&mut self) {
        self.paths.clear();
        self._fills.clear();
    }

    pub fn add_path(&mut self, path : VectorPath) {
        self.paths.push(path)
    }

    pub fn get_paths(&self) -> &Vec<VectorPath> {
        &self.paths
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

    fn reset(&mut self, color : &Color32) {
        for i in 0..VIEWPORT_PIXELS {
            self.pixels[i] = *color;
            self.instruction_indexes[i] = None;
        }
    }

    fn isolate_instruction_pixels(&mut self, index : usize, _sub_index : Option<usize>, mask_color : Color32) {
        for i in 0..VIEWPORT_PIXELS {
            if let Some(inst) = self.instruction_indexes[i] {
                if inst.base_index as usize != index {
                    self.pixels[i] = mask_color;
                }
            }
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

    pub fn get_pixel_instruction(&self, x : usize, y : usize) -> Result<Option<InstructionIndex>, AgiError> {
        let index = y * VIEWPORT_WIDTH + x;
        if index >= VIEWPORT_PIXELS {
            Err(AgiError::Render(format!("Pixel location ({x},{y}) out of range!")))
        } else {
            Ok(self.instruction_indexes[index])
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

pub fn render_to_buffers(
    instructions : &[DerivedPicRenderInstruction],
    render_options : &RenderOptions,
    pic_buffer : &mut Option<&mut PixelBuffer>,
    pri_buffer : &mut Option<&mut PixelBuffer>,
    pic_vectors : &mut Option<&mut ShapeBuffer>) -> Result<(), AgiError> {

    // Clear pixel buffers
    if let Some(pic_buffer) = pic_buffer {
        pic_buffer.reset(&get_color(PIC_BUFFER_BASE_COLOR));
    }

    if let Some(pri_buffer) = pri_buffer {
        pri_buffer.reset(&get_color(PRI_BUFFER_BASE_COLOR));
    }

    if let Some(pic_vectors) = pic_vectors {
        pic_vectors.clear();
    }

    let mut pic_color = Some(PIC_BUFFER_BASE_COLOR);
    let mut pri_color = Some(PRI_BUFFER_BASE_COLOR);

    let latest_instruction_index = instructions.len() - 1;

    // Option aliases
    let only_latest_instruction = render_options.render_only_selected_instruction;
    let show_fill_outlines = render_options.show_fill_outlines;

    // Actual rendering
    for (instruction_index, instruction) in instructions.iter().enumerate() {
        let render_instruction = !only_latest_instruction || (only_latest_instruction && instruction_index == latest_instruction_index);
        match instruction {
            DerivedPicRenderInstruction::SetColor(_, buffer_type, color) => {
                match buffer_type {
                    PictureBufferType::Picture => pic_color = *color,
                    PictureBufferType::Priority => pri_color = *color
                }
            },
            DerivedPicRenderInstruction::DrawLines(_, lines) => {
                draw_pixel_lines(lines, pic_buffer, pic_color, pri_buffer, pri_color, instruction_index)?;

                // For vectors, only place the latest instruction if requested
                if render_instruction {
                    if let (Some(pic_vectors), Some(pic_color)) = (&mut *pic_vectors, pic_color) {
                        pic_vectors.add_path(VectorPath::from_point_list(&lines, pic_color));
                    }
                }
            },
            DerivedPicRenderInstruction::Fill(_, points) => {
                let (pic_edges, pri_edges) = pixel_fill(points, pic_buffer, pic_color, pri_buffer, pri_color, instruction_index)?;
                
                // For vectors, only place the latest instruction if requested
                if render_instruction {
                    if let (Some(pic_vectors), Some(pic_color)) = (&mut *pic_vectors, pic_color) {
                        // TODO: Move this out to its own function
                        for pic_edge_list in pic_edges {
                            for pic_edge in pic_edge_list {
                                let line = pic_edge.to_line(&instructions)?;

                                if show_fill_outlines {
                                    pic_vectors.add_path(VectorPath::from_line(line, pic_color));
                                }
                            }
                        }
                    }
                }
            },
            DerivedPicRenderInstruction::Unimplemented(_orignal_inst) => {
                // TODO: Log?
            }
        }
    }

    if only_latest_instruction {
        // Mask out pixels from other instructions
        if let Some(pic_buffer) = pic_buffer {
            pic_buffer.isolate_instruction_pixels(latest_instruction_index, None, get_color(PIC_BUFFER_BASE_COLOR));
        }
        
        if let Some(pri_buffer) = pri_buffer {
            pri_buffer.isolate_instruction_pixels(latest_instruction_index, None, get_color(PRI_BUFFER_BASE_COLOR));
        }
        
    }

    Ok(())
}

fn draw_pixel_lines(lines : &Vec<PosU8>, pic_buffer : &mut Option<&mut PixelBuffer>, pic_color : Option<u8>, pri_buffer : &mut Option<&mut PixelBuffer>, pri_color : Option<u8>, instruction_index : usize) -> Result<(), AgiError> {
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
            for (point_index, line) in lines.iter().enumerate().skip(1) {
                let line_index = point_index - 1;

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

fn pixel_fill(points : &[PosU8], pic_buffer : &mut Option<&mut PixelBuffer>, pic_color : Option<u8>, pri_buffer : &mut Option<&mut PixelBuffer>, pri_color : Option<u8>, instruction_index : usize) -> Result<(Vec<Vec<FillEdge>>, Vec<Vec<FillEdge>>), AgiError> {

    let mut pic_edges : Vec<Vec<FillEdge>> = vec![];
    let mut pri_edges : Vec<Vec<FillEdge>> = vec![];

    for (sub_index, point) in points.iter().enumerate() {
        if let (Some(pic_buffer), Some(pic_color)) = (&mut *pic_buffer, pic_color) {
            pic_edges.push(pixel_fill_specific_buffer(*point, pic_color, &PictureBufferType::Picture, pic_buffer, instruction_index, sub_index)?);
        }

        if let (Some(pri_buffer), Some(pri_color)) = (&mut *pri_buffer, pri_color) {
            pri_edges.push(pixel_fill_specific_buffer(*point, pri_color, &PictureBufferType::Priority, pri_buffer, instruction_index, sub_index)?);
        }
    }

    Ok((pic_edges, pri_edges))
}

fn pixel_fill_specific_buffer(point : PosU8, color : u8, buffer_type : &PictureBufferType, buffer : &mut PixelBuffer, instruction_index : usize, sub_index : usize) -> Result<Vec<FillEdge>, AgiError> {

    let default_color = match &buffer_type {
        PictureBufferType::Picture => PIC_BUFFER_BASE_COLOR,
        PictureBufferType::Priority => PRI_BUFFER_BASE_COLOR
    };

    // Use these to track what is touched by the fill
    let mut fill_edges : Vec<FillEdge> = vec![];

    if color == default_color {
        // Filling with the default color is a no-op 
        return Ok(fill_edges);
    }

    // Do our fill
    let mut fill_queue = VecDeque::from([(point.x as usize, point.y as usize)]);

    let (mut top, mut right, mut bottom, mut left) = (false, false, false, false);

    while !fill_queue.is_empty() {
        let (cur_x, cur_y) = fill_queue.pop_front().unwrap();

        if buffer.get_pixel(cur_x, cur_y).unwrap() == get_color(default_color) {
            // Fill this and add our surroundings
            buffer.set_pixel(cur_x, cur_y, Some(color), InstructionIndex::new_sub(instruction_index, sub_index))?;

            if cur_x < VIEWPORT_WIDTH - 1 { 
                fill_queue.push_back((cur_x+1, cur_y));
            } else {
                right = true;
            }

            if cur_x > 0 { 
                fill_queue.push_back((cur_x-1, cur_y));
            } else {
                left = true;
            }

            if cur_y < VIEWPORT_HEIGHT - 1 {
                fill_queue.push_back((cur_x, cur_y+1));
            } else {
                bottom = true;
            }

            if cur_y > 0 {
                fill_queue.push_back((cur_x, cur_y-1));
            } else {
                top = true;
            }
        } else {
            if let Some(inst) = buffer.get_pixel_instruction(cur_x, cur_y)? {
                if inst.base_index as usize != instruction_index {
                    // Get the line for this instruction
                    // I'm going to assume since these buffers should be small that a search of the vec will be faster
                    // than using a hash set
                    // TODO: Measure?
                    let edge = FillEdge::Line(inst);
                    if !fill_edges.contains(&edge) {
                        fill_edges.push(edge);
                    }
                }
            }
        }
    }

    if top {
        fill_edges.push(FillEdge::TopBorder);
    }
    if right {
        fill_edges.push(FillEdge::RightBorder);
    }
    if bottom {
        fill_edges.push(FillEdge::BottomBorder);
    }
    if left {
        fill_edges.push(FillEdge::LeftBorder)
    }

    /*
    if top { 
        border_lines.insert(line_u8(0u8,0u8, (VIEWPORT_WIDTH-1) as u8, 0u8 )); 
    }
    if right { 
        border_lines.insert(line_u8((VIEWPORT_WIDTH-1) as u8, 0u8, (VIEWPORT_WIDTH-1) as u8, (VIEWPORT_HEIGHT-1) as u8 )); 
    }
    if bottom { 
        border_lines.insert(line_u8((VIEWPORT_WIDTH-1) as u8, (VIEWPORT_HEIGHT-1) as u8 , 0 as u8, (VIEWPORT_HEIGHT-1) as u8 )); 
    }
    if left { 
        border_lines.insert(line_u8(0u8,(VIEWPORT_HEIGHT-1) as u8, 0u8, 0u8 )); 
    }*/

    Ok(fill_edges)
}
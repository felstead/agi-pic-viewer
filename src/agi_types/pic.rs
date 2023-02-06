use std::fmt::Display;
use crate::*;

#[derive(Debug)]
pub enum PictureBufferType {
    Picture,
    Priority
}

#[derive(Debug)]
pub struct PicResource {
    instructions : Vec<DerivedPicRenderInstruction>
}

impl PicResource {
    pub fn new(raw_data : &Vec<u8>) -> Result<Self, AgiError> {
        // Read the instructions
        let mut offset = 0usize;

        let mut resource = PicResource { 
            instructions: vec![],
        };

        while offset < raw_data.len() {
            let (instruction, next_offset) = DerivedPicRenderInstruction::create_from_vec(raw_data, offset);
            resource.instructions.push(instruction);
            offset = next_offset;
        }

        Ok(resource)
    }


    pub fn get_instructions(&self) -> &Vec<DerivedPicRenderInstruction> {
        &self.instructions
    }
}

#[derive(Debug, Clone)]
pub enum PicRenderInstruction {
    SetPicColorAndEnablePicDraw,
    DisablePicDraw,
    SetPriColorAndEnablePriDraw,
    DisablePriDraw,
    DrawYCorner,
    DrawXCorner,
    AbsLine,
    RelLine,
    Fill,
    SetPenSizeAndStyle,
    PlotWithPen,
    EndInstruction,
    Unknown
}

impl Display for PicRenderInstruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self)
    }
}

#[derive(PartialEq, Eq, Debug)]
enum CornerLineStartDirection {
    StartOnX,
    StartOnY
}

#[derive(Debug)]
pub enum DerivedPicRenderInstruction {
    SetColor(PicRenderInstruction, PictureBufferType, Option<u8>),
    DrawLines(PicRenderInstruction, Vec<PosU8>),
    Fill(PicRenderInstruction, Vec<PosU8>),
    Unimplemented(PicRenderInstruction)
}

impl Display for DerivedPicRenderInstruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Self::SetColor(inst, _, color) => {
                match color {
                    Some(color) => format!("{:?} ({})", inst, get_color_str(*color)),
                    None => format!("{:?}", inst)
                }
            },
            Self::DrawLines(inst, points) |
            Self::Fill(inst, points) => {
                let points_str = points.iter().map(|l| format!("({},{})", l.x, l.y)).collect::<Vec<String>>().join(", ");
                format!("{:?} {}", inst, points_str)
            },
            Self::Unimplemented(inst) => {
                format!("{:?}", inst)
            }
        };

        f.write_str(str.as_str())
    }
}

impl DerivedPicRenderInstruction {
    pub fn create_from_vec(raw_data : &[u8], offset : usize) -> (DerivedPicRenderInstruction, usize) {
        let instruction = raw_data[offset];

        // Extract the arguments
        let mut current_offset = offset + 1;
        while current_offset < raw_data.len() && raw_data[current_offset] & 0xF0 != 0xF0 {
            current_offset += 1;
        }
        
        let arguments = &raw_data[offset+1..current_offset];

        let derived_instruction = match instruction {
            0xF0 => Self::SetColor(PicRenderInstruction::SetPicColorAndEnablePicDraw, PictureBufferType::Picture, Some(arguments[0])),
            0xF1 => Self::SetColor(PicRenderInstruction::DisablePicDraw, PictureBufferType::Picture, None),
            0xF2 => Self::SetColor(PicRenderInstruction::SetPriColorAndEnablePriDraw, PictureBufferType::Priority, Some(arguments[0])),
            0xF3 => Self::SetColor(PicRenderInstruction::DisablePriDraw, PictureBufferType::Priority, None),
            0xF4 => Self::DrawLines(PicRenderInstruction::DrawYCorner, Self::generate_corner_lines(arguments, CornerLineStartDirection::StartOnY)),
            0xF5 => Self::DrawLines(PicRenderInstruction::DrawXCorner, Self::generate_corner_lines(arguments, CornerLineStartDirection::StartOnX)),
            0xF6 => Self::DrawLines(PicRenderInstruction::AbsLine, Self::generate_point_pairs(arguments)),
            0xF7 => Self::DrawLines(PicRenderInstruction::RelLine, Self::generate_rel_lines(arguments)),
            0xF8 => Self::Fill(PicRenderInstruction::Fill, Self::generate_point_pairs(arguments)),
            0xF9 => Self::Unimplemented(PicRenderInstruction::SetPenSizeAndStyle),
            0xFA => Self::Unimplemented(PicRenderInstruction::PlotWithPen),
            0xFF => Self::Unimplemented(PicRenderInstruction::EndInstruction),
            _ => Self::Unimplemented(PicRenderInstruction::Unknown)
        };

        (derived_instruction, current_offset)
    }

    fn generate_corner_lines(arguments : &[u8], start_dir : CornerLineStartDirection) -> Vec<PosU8> {
        let mut result : Vec<PosU8> = vec![];
        
        if arguments.len() >= 2 {
            result.push(PosU8::new(arguments[0], arguments[1]));
            
            let (mut x, mut y) = (arguments[0], arguments[1]);
            let mut direction_is_x = start_dir == CornerLineStartDirection::StartOnX;
    
            for arg in arguments.iter().skip(2) {
                if direction_is_x {
                    x = *arg;
                } else {
                    y = *arg;
                }
    
                result.push(PosU8::new(x, y));

                direction_is_x = !direction_is_x;
            }
        } else {
            // TODO: Log error
        }

        result

    }

    fn generate_point_pairs(arguments : &[u8]) -> Vec<PosU8> {
        let mut result : Vec<PosU8> = vec![];
        
        if !arguments.is_empty() && arguments.len() % 2 == 0 {
            result.reserve_exact(arguments.len() / 2);

            for i in (0..arguments.len()).step_by(2) {
                result.push(PosU8::new(arguments[i], arguments[i+1]))
            }
        } else {
            // TODO: Error
        }

        result
    }

    fn generate_rel_lines(arguments : &[u8]) -> Vec<PosU8> {
        let mut result : Vec<PosU8> = vec![];
        
        if arguments.len() >= 2 {
            // Convert the relative arguments to absolute
            result.push(PosU8::new(arguments[0], arguments[1]));

            let (mut x, mut y) = (arguments[0], arguments[1]);
            for arg in arguments.iter().skip(2) {
                let sign_x = if 0x80 & arg > 0 { -1i8 } else { 1i8 };
                let sign_y = if 0x08 & arg > 0 { -1i8 } else { 1i8 };

                let disp_x = sign_x * ((arg & 0x70) >> 4) as i8;
                let disp_y = sign_y * (arg & 0x07) as i8;

                let (x1, y1) = ((x as i16 + disp_x as i16) as u8, (y as i16 + disp_y as i16) as u8);
                result.push(PosU8::new(x1, y1));

                x = x1;
                y = y1;
            }
        } else {
            // TODO: Error
        }

        result
    }
}

pub fn get_color_str(agi_color : u8) -> &'static str {
    // From here: https://moddingwiki.shikadi.net/wiki/EGA_Palette
    match agi_color {
        0x00 => "black",
        0x01 => "blue",
        0x02 => "green",
        0x03 => "cyan",
        0x04 => "red",
        0x05 => "magenta",
        0x06 => "brown",
        0x07 => "light gray",
        0x08 => "dark gray",
        0x09 => "light blue",
        0x0A => "light green",
        0x0B => "light cyan",
        0x0C => "light red",
        0x0D => "light magenta",
        0x0E => "yellow",
        0x0F => "white",
        _ => "INVALID"
    }
}
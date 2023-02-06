use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgiError {
    #[error("IO error")]
    Io(#[from] std::io::Error),
    #[error("Parse error")]
    Parse(String),
    #[error("Render error")]
    Render(String)
}

pub const VIEWPORT_WIDTH : usize = 160;
pub const VIEWPORT_HEIGHT : usize = 168;
pub const VIEWPORT_PIXELS : usize = VIEWPORT_WIDTH * VIEWPORT_HEIGHT;

pub const PIC_BUFFER_BASE_COLOR : u8 = 0xF; // White
pub const PRI_BUFFER_BASE_COLOR : u8 = 0x4; // Red

#[derive(Debug, Copy, Clone)]
pub struct PosU8 {
    pub x : u8,
    pub y : u8
}

impl PosU8 {
    pub fn new(x : u8, y : u8) -> Self {
        Self { x, y }
    }
}
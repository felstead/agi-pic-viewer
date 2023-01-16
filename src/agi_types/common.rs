use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgiError {
    #[error("IO error")]
    IoError(#[from] std::io::Error),
    #[error("Parse error")]
    ParseError(String)
}

pub const VIEWPORT_WIDTH : usize = 160;
pub const VIEWPORT_HEIGHT : usize = 168;
pub const VIEWPORT_PIXELS : usize = VIEWPORT_WIDTH * VIEWPORT_HEIGHT;

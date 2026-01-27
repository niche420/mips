use crate::ps1::Ps1Frame;

pub struct CpuFrame {
    pub pixels: Vec<u32>,
    pub width: u32,
    pub height: u32,
}

#[cfg(feature = "ps1")]
impl From<Ps1Frame> for CpuFrame {
    fn from(frame: Ps1Frame) -> Self {
        Self {
            width: frame.width,
            height: frame.height,
            pixels: frame.pixels,
        }
    }
}
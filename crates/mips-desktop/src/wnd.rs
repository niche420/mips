pub mod canvas;

use std::ops::Deref;
use sdl3::Sdl;
use tracing::instrument::WithSubscriber;
use crate::error::AppResult;

pub struct Window {
    ctx: Sdl,
    wnd: sdl3::video::Window,
}

impl Window {
    pub fn new() -> AppResult<Self> {
        let ctx = sdl3::init()?;
        let wnd = ctx.video()?.window("MIPS", 1280, 720)
            .position_centered()
            .build()
            .unwrap();

        Ok(Window {
            ctx,
            wnd,
        })
    }
    
    pub fn ctx(&self) -> &Sdl { &self.ctx }
    
    pub fn width(&self) -> u32 {
        self.wnd.size_in_pixels().0
    }
    
    pub fn height(&self) -> u32 {
        self.wnd.size_in_pixels().1
    }
}


impl Deref for Window {
    type Target = sdl3::video::Window;
    fn deref(&self) -> &Self::Target {
        &self.wnd
    }
}

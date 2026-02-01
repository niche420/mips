use std::collections::HashMap;
use std::env;
use std::ops::Deref;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use sdl3::event::EventType;
use sdl3::{EventPump, Sdl};
use tracing::info;
use mips_core::ConsoleManager;
use mips_core::input::{DeviceType, InputConfig};
use crate::error::AppResult;
use crate::{audio, evt, input, wnd};
use crate::input::InputDeviceMap;
use crate::ui::Ui;
use crate::wnd::canvas::Canvas;
use crate::wnd::Window;

pub struct App {
    pub(crate) ctx: Sdl,
    pub(crate) wnd: wnd::Window,
    ui: Ui,
    
    mips: ConsoleManager,
    pub running: bool,
    
    ports: [input::Port; 2],
    pub controllers: InputDeviceMap,
}

impl App {
    pub fn new() -> AppResult<Self> {
        let ctx = sdl3::init()?;
        let wnd = Window::new(&ctx)?;
        let ui = Ui::new(&wnd)?;

        let sys_dir = env::current_dir().unwrap();
        let mut mips = ConsoleManager::new();
        mips.load_game(sys_dir.as_path(), Some("Silent Hill (USA).cue"))?;
        
        Ok(App {
            mips,
            ctx,
            wnd,
            ui,
            running: true,
            ports: [
                input::Port::new(),
                input::Port::new(),
            ],
            controllers: InputDeviceMap::new(),
        })
    }

    pub fn on_resize(&mut self, w: i32, h: i32) {

    }

    pub fn run(&mut self) {
        use std::ops::Deref;
        use sdl3::pixels::PixelFormat;
        use sdl3::sys::pixels::SDL_PixelFormat;

        let mut canvas = Canvas::from(&self.wnd);

        let texture_creator = canvas.deref().texture_creator();

        if let Some(port) = self.ports.get_mut(0) {
            let config = InputConfig::from("assets/config/profile.input.ini".as_ref());
            port.connect_controller(self.controllers.keyboard());
            port.load_config(config);
            self.mips.connect_device(0, DeviceType::Keyboard);
        }

        // Audio stream: must be created and used in main thread
        let audio = self.ctx.audio().unwrap();
        let audio_device = audio::Device::from(audio);
        let mut audio_stream = audio::StreamWithCallback::from(audio_device);
        audio_stream.resume();

        let mut texture = texture_creator
            .create_texture_streaming(
                unsafe { PixelFormat::from_ll(SDL_PixelFormat::XRGB8888) },
                self.wnd.width(),
                self.wnd.height(),
            )
            .expect("Failed to create texture");

        const FRAME_TIME: Duration = Duration::from_nanos(16_666_667); // ~60 FPS

        // Main loop
        while self.running {
            let frame_start = Instant::now();

            evt::poll(self);

            audio_stream.enqueue(self.mips.get_audio_samples());
            self.mips.clear_audio_samples();

            self.mips.update();

            // Render video frame
            self.ui.render(&mut self.ctx, &self.wnd);
            if let Some(frame) = self.mips.get_frame() {
                if frame.width != self.wnd.width() || frame.height != self.wnd.height() {
                    texture = texture_creator
                        .create_texture_static(
                            unsafe { PixelFormat::from_ll(SDL_PixelFormat::XRGB8888) },
                            frame.width,
                            frame.height,
                        )
                        .expect("Failed to create texture");
                }

                let pitch = frame.width as usize * size_of::<u32>(); // bytes per row
                let pixel_bytes: &[u8] = bytemuck::cast_slice(frame.pixels.as_slice());

                texture.update(None, pixel_bytes, pitch).unwrap();
                self.ui.set_game_frame(&texture);
            }

            let btn_states = self.ports[0].inputs();
            self.mips.handle_inputs(btn_states);
            self.mips.refresh_devices();

            // Frame timing
            let elapsed = frame_start.elapsed();
            if elapsed < FRAME_TIME {
                std::thread::sleep(FRAME_TIME - elapsed);
            }
        }
    }
}
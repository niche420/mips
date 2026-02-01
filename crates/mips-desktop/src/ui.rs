mod topbar;
mod component;
mod games_list;

use std::ops::Deref;
use imgui_sdl3::ImGuiSdl3;
use sdl3::event::Event;
use sdl3::gpu::{ColorTargetInfo, Device, Filter, LoadOp, SamplerAddressMode, SamplerCreateInfo, SamplerMipmapMode, ShaderFormat, StoreOp};
use sdl3::pixels::Color;
use sdl3::{EventPump, Sdl};
use sdl3::render::Texture;
use crate::error::AppResult;
use crate::ui::component::UiComponent;
use crate::ui::games_list::GamesList;
use crate::ui::topbar::Topbar;
use crate::wnd::Window;

pub struct Ui {
    device: Device,
    ctx: ImGuiSdl3,
    components: Vec<Box<dyn UiComponent>>,
    sampler: sdl3::gpu::Sampler,
    game_frame: Option<>
}

impl Ui {
    pub fn new(window: &Window) -> AppResult<Self> {
        let device = Device::new(ShaderFormat::SPIRV, true)?.with_window(&window.deref())?;

        // create platform and renderer
        let mut ctx = ImGuiSdl3::new(&device, &window, |ctx| {
            // disable creation of files on disc
            ctx.set_ini_filename(None);
            ctx.set_log_filename(None);

            ctx.fonts()
                .add_font(&[imgui::FontSource::DefaultFontData { config: None }]);
        });

        let components: Vec<Box<dyn UiComponent>> = vec![
            Box::new(Topbar::new()),
            Box::new(GamesList::new())
        ];

        let sampler: sdl3::gpu::Sampler = device.create_sampler(
            SamplerCreateInfo::new()
                .with_min_filter(Filter::Linear)
                .with_mag_filter(Filter::Linear)
                .with_mipmap_mode(SamplerMipmapMode::Linear)
                .with_address_mode_u(SamplerAddressMode::Repeat)
                .with_address_mode_v(SamplerAddressMode::Repeat)
                .with_address_mode_w(SamplerAddressMode::Repeat),
        )?;

        Ok(Self {
            device,
            ctx,
            components,
            sampler
        })
    }

    pub fn handle_event(&mut self, event: Event) {
        self.ctx.handle_event(&event);
    }

    pub fn render(&mut self, ctx: &mut Sdl, window: &Window) -> AppResult<()> {
        let mut command_buffer = self.device.acquire_command_buffer()?;
        if let Ok(swapchain) = command_buffer.wait_and_acquire_swapchain_texture(&window) {
            let color_targets = [ColorTargetInfo::default()
                .with_texture(&swapchain)
                .with_load_op(LoadOp::CLEAR)
                .with_store_op(StoreOp::STORE)
                .with_clear_color(Color::RGB(128, 128, 128))];

            let event_pump = ctx.event_pump()?;
            self.ctx.render(
                ctx,
                &self.device,
                &window,
                &event_pump,
                &mut command_buffer,
                &color_targets,
                |ui| {
                    for component in &mut self.components {
                        component.draw(ui);
                    }

                    if let Some(game_frame) = self.game_frame {
                        let rust_logo_tex = ui.push_texture(game_frame, self.sampler);
                    }
                },
            );

            Ok(command_buffer.submit()?)
        } else {
            println!("Swapchain unavailable, cancel work");
            Ok(command_buffer.cancel())
        }
    }
}
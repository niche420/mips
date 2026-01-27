use std::ops::Deref;
use sdl3::event::{Event, EventPollIterator, WindowEvent};
use sdl3::EventPump;
use mips_core::input::{ButtonState, DeviceType};
use crate::app::App;
use crate::error::AppResult;
use crate::input::device::InputDevice;

type EventHandler = fn(e: Event);

pub trait Observer {
    fn subscribe(&mut self, event_pump: &mut EventPump);
    fn unsubscribe(&mut self, event_pump: &mut EventPump);
}

pub fn poll(app: &mut App) -> AppResult<()> {
    let mut pump = app.wnd.ctx().event_pump()?;
    let events = pump.poll_iter();
    for e in events {
        match e {
            Event::Quit {..} => {
                app.running = false;
            },
            Event::KeyDown { keycode: Some(keycode), ..} => {
                app.controllers.push_keycode(ButtonState::Pressed, keycode);
            },
            Event::KeyUp { keycode: Some(keycode), ..} => {
                app.controllers.push_keycode(ButtonState::Released, keycode);
            },
            Event::JoyDeviceAdded { which, .. } => {
                let controller = InputDevice::new(DeviceType::DualShock, which);
                app.controllers.insert_controller(controller);
            },
            Event::JoyDeviceRemoved { which, .. } => {
                app.controllers.remove_controller(DeviceType::DualShock, which);
            },
            Event::JoyButtonDown {which, button_idx, ..} => {
                app.controllers.push_gamepad_input(ButtonState::Pressed, which, button_idx);
            },
            Event::JoyButtonUp {which, button_idx, ..} => {
                app.controllers.push_gamepad_input(ButtonState::Released, which, button_idx);
            },
            Event::Window { win_event, ..} => {
                match win_event {
                    WindowEvent::Resized(w, h) => {
                        //app.wnd.on_resize(w, h);
                    },
                    _ => {}
                }
            }
            _ => {}
        }
    }
    
    Ok(())
}

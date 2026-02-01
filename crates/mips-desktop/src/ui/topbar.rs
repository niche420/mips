use std::ops::Deref;
use crate::ui::component::UiComponent;
use crate::ui::Ui;

pub struct Topbar {
    
}

impl Topbar {
    pub fn new() -> Topbar {
        Topbar {
            
        }
    }
    
}

impl UiComponent for Topbar {
    fn draw(&mut self, ctx: &mut imgui::Ui) {
        if let Some(bar) = ctx.begin_main_menu_bar() {
            ctx.menu_item("File");
        }
    }
}
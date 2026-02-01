use crate::ui::component::UiComponent;

pub struct GamesList {

}

impl GamesList {
    pub fn new() -> GamesList {
        GamesList {}
    }
}

impl UiComponent for GamesList {
    fn draw(&mut self, ctx: &mut imgui::Ui) {
        
    }
}
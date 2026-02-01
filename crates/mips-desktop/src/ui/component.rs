pub trait UiComponent {
    fn draw(&mut self, ctx: &mut imgui::Ui);
}
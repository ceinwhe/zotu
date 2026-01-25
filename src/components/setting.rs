use gpui::*;

pub struct Setting;

impl Render for Setting {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().flex().flex_col().child("设置页面").child(
            div()
                .flex()
                .flex_row()
                .gap_2()
                .cursor_pointer()
                .child(svg().path("folder.svg").text_color(black()).size_6())
                .child("媒体文件"),
        )
    }
}

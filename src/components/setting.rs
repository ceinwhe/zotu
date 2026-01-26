use gpui::*;
use crate::config::Config;
pub struct Setting;

impl Render for Setting {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .justify_start()
                    .text_xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("设置"),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_4()
                    .cursor_pointer()
                    .child(svg().path("folder.svg").text_color(black()).size_6())
                    .child("媒体文件")
                    .child(cx.global::<Config>().media_file.music_directory.clone())
            )
    }
}

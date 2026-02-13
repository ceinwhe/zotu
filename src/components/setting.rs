use gpui::*;
use crate::config::Config;

const SETTING_ITEM_HEIGHT: f32 = 50.0;

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
                    .mb_4()
                    .text_2xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("设置"),
            )
            .child(
                div()
                    .h(Pixels::from(SETTING_ITEM_HEIGHT))
                    .mr_5() 
                    .flex()
                    .flex_row()
                    .gap_4()
                    .cursor_pointer()
                    .child(svg().path("svg/folder.svg").text_color(black()).size_6())
                    .child("媒体文件")
                    .child(cx.global::<Config>().media_file.music_directory.clone())
            )
    }
}

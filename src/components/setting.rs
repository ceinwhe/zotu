use gpui::*;


pub struct Setting;

enum SettingMessage {
    File
}
impl EventEmitter<SettingMessage> for Setting {}  


impl Render for Setting {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .child("设置页面")
            .child(
                div()
                    .flex()
                    .flex_row()
                    .child(svg().path("folder.svg").text_color(black()).size_6())
                    .child("媒体文件")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|_this,_evt,_window,cx|{
                            cx.emit(SettingMessage::File);
                        })
                    )
            )
    }
}
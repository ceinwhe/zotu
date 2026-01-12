use gpui::prelude::*;
use gpui::*;

pub const TITLEBAR_HEIGHT: f32 = 50.0;

pub struct TitleBar;

impl Render for TitleBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .h(px(TITLEBAR_HEIGHT))
            .flex()
            .flex_row()
            .justify_end()
            .bg(rgb(0xFAFAFA))
            //拖拽区域
            .child(
                div()
                    .h_full()
                    .flex_grow()
                    .window_control_area(WindowControlArea::Drag)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|_this, _event, window, _cx| {
                            window.start_window_move();
                        }),
                    ),
            )
            //控制按钮区域
            .child(
                div()
                    .h_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_4()
                    .child(
                        svg()
                            .path("minus.svg")
                            .w(px(30.0))
                            .h(px(30.0))
                            .text_color(black())
                            .window_control_area(WindowControlArea::Min)
                            .cursor_pointer()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|_this, _event, window, _cx| {
                                    window.minimize_window();
                                }),
                            ),
                    )
                    .child(
                        svg()
                            .path("stack.svg")
                            .w(px(30.0))
                            .h(px(30.0))
                            .text_color(black())
                            .cursor_pointer()
                            .window_control_area(WindowControlArea::Max)
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|_this, _event, window, _cx| {
                                    window.zoom_window();
                                }),
                            )
                    )
                    .child(
                        svg()
                            .path("close.svg")
                            .w(px(30.0))
                            .h(px(30.0))
                            .text_color(black())
                            .window_control_area(WindowControlArea::Close)
                            .cursor_pointer()
                            .hover(|style|style.bg(rgb(0xFF6467)))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|_this, _event, window, _cx| {
                                    window.remove_window();
                                }),
                            )
                    )
            )
    }
}

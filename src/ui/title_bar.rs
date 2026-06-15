use gpui::*;

use crate::{application::AppController, theme::*};

use super::SearchBox;

pub struct TitleBar {
    search_box: Entity<SearchBox>,
}

impl TitleBar {
    pub fn new(controller: Entity<AppController>, cx: &mut Context<Self>) -> Self {
        Self {
            search_box: cx.new(|cx| SearchBox::new(controller, cx)),
        }
    }
}

impl Render for TitleBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .h(px(TITLEBAR_HEIGHT))
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .bg(bg_app())
            .child(self.search_box.clone())
            .child(
                div()
                    .h_full()
                    .flex_1()
                    .window_control_area(WindowControlArea::Drag)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|_this, _event, window, _cx| {
                            window.start_window_move();
                        }),
                    ),
            )
            .child(
                div()
                    .h_full()
                    .w_auto()
                    .flex()
                    .flex_row()
                    .child(
                        svg()
                            .path("svg/minus.svg")
                            .w(px(30.0))
                            .h(px(30.0))
                            .text_color(text_secondary())
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
                            .path("svg/stack.svg")
                            .w(px(30.0))
                            .h(px(30.0))
                            .text_color(text_secondary())
                            .cursor_pointer()
                            .window_control_area(WindowControlArea::Max)
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|_this, _event, window, _cx| {
                                    window.zoom_window();
                                }),
                            ),
                    )
                    .child(
                        svg()
                            .path("svg/close.svg")
                            .w(px(30.0))
                            .h(px(30.0))
                            .text_color(text_secondary())
                            .window_control_area(WindowControlArea::Close)
                            .cursor_pointer()
                            .hover(|style| style.bg(accent_red()))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|_this, _event, window, _cx| {
                                    window.remove_window();
                                }),
                            ),
                    ),
            )
    }
}

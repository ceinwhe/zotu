use gpui::*;

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.open_window(WindowOptions::default(), |_window, cx| {
            cx.new(|_cx| HelloWorld)
        })
        .unwrap();
    });
}

struct HelloWorld;

impl Render for HelloWorld {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .bg(rgba(0xFAFAFA))
            .w_full()
            .h_full()
            .flex()
            .justify_center()
            .items_center()
            .child(div().child("关").bg(yellow()).on_mouse_down(
                MouseButton::Left,
                _cx.listener(|_a, _b, c, _d| {
                    c.remove_window();
                }),
            ))
            .child(
                div()
                    .child("拖")
                    .bg(blue())
                    .window_control_area(WindowControlArea::Drag)
                    .w(px(300.0))
                    .h(px(500.0))
                    .on_mouse_down(
                        MouseButton::Left,
                        _cx.listener(|_this, _event, window, _cx| {
                            window.start_window_move();
                        }),
                    ),
            )
            .child(div().child("变").bg(black()).on_mouse_down(
                MouseButton::Left,
                _cx.listener(|_this, _event, window, _cx| {
                    window.toggle_fullscreen();
                }),
            ))
    }
}

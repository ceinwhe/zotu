use gpui::*;

use crate::ui::search::{ClearSearchEvent, SearchBox, SearchEvent};

pub const TITLEBAR_HEIGHT: f32 = 70.0;

pub struct TitleBar {
    /// 搜索框组件
    search_box: Entity<SearchBox>,
}

impl TitleBar {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let search_box = cx.new(|cx| SearchBox::new(cx));

        // 转发搜索事件
        cx.subscribe(
            &search_box,
            |_this, _search_box, event: &SearchEvent, cx| {
                cx.emit(event.clone());
            },
        )
        .detach();

        // 转发清除搜索事件
        cx.subscribe(
            &search_box,
            |_this, _search_box, _event: &ClearSearchEvent, cx| {
                cx.emit(ClearSearchEvent);
            },
        )
        .detach();

        Self { search_box }
    }
}

impl EventEmitter<SearchEvent> for TitleBar {}
impl EventEmitter<ClearSearchEvent> for TitleBar {}

impl Render for TitleBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .h(px(TITLEBAR_HEIGHT))
            .flex()
            .flex_row()
            .items_center()
            .justify_around()
            .bg(rgb(0xFAFAFA))
            .px_4()
            // 左侧拖拽区域
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
            // 中间搜索框
            .child(self.search_box.clone())
            // 右侧拖拽区域
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
            // 控制按钮区域
            .child(
                div()
                    .h_full()
                    .w_auto()
                    .gap_4()
                    .flex()
                    .flex_row()
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
                            ),
                    )
                    .child(
                        svg()
                            .path("close.svg")
                            .w(px(30.0))
                            .h(px(30.0))
                            .text_color(black())
                            .window_control_area(WindowControlArea::Close)
                            .cursor_pointer()
                            .hover(|style| style.bg(rgb(0xFF6467)))
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

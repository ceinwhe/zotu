use gpui::{prelude::FluentBuilder, *};

/// 搜索事件
#[derive(Clone)]
pub struct SearchEvent {
    pub query: String,
}

/// 清除搜索事件
#[derive(Clone, Copy)]
pub struct ClearSearchEvent;

/// 搜索框组件
pub struct SearchBox {
    /// 当前搜索关键词
    search_query: String,
    /// 搜索框是否获得焦点
    is_focused: bool,
    /// 焦点句柄
    focus_handle: FocusHandle,
}

impl SearchBox {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            search_query: String::new(),
            is_focused: false,
            focus_handle: cx.focus_handle(),
        }
    }

    /// 处理键盘输入
    fn handle_key_input(&mut self, input: &str, cx: &mut Context<Self>) {
        self.search_query.push_str(input);
        if !self.search_query.is_empty() {
            cx.emit(SearchEvent {
                query: self.search_query.clone(),
            });
        }
        cx.notify();
    }

    /// 处理退格键
    fn handle_backspace(&mut self, cx: &mut Context<Self>) {
        self.search_query.pop();
        if self.search_query.is_empty() {
            cx.emit(ClearSearchEvent);
        } else {
            cx.emit(SearchEvent {
                query: self.search_query.clone(),
            });
        }
        cx.notify();
    }

    /// 清除搜索
    fn clear_search(&mut self, cx: &mut Context<Self>) {
        self.search_query.clear();
        cx.emit(ClearSearchEvent);
        cx.notify();
    }
}

impl EventEmitter<SearchEvent> for SearchBox {}
impl EventEmitter<ClearSearchEvent> for SearchBox {}

impl Render for SearchBox {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_query = !self.search_query.is_empty();
        let display_text = if self.search_query.is_empty() {
            "搜索歌曲、歌手、专辑...".to_string()
        } else {
            self.search_query.clone()
        };
        let is_placeholder = self.search_query.is_empty();
        let is_focused = self.focus_handle.is_focused(window);

        div()
            .id("search-container")
            .key_context("SearchInput")
            .w(px(300.0))
            .h(px(36.0))
            .flex()
            .flex_row()
            .items_center()
            .px_3()
            .bg(rgb(0xF3F4F6))
            .rounded_lg()
            .border_1()
            .border_color(if is_focused {
                rgb(0x3B82F6)
            } else {
                rgb(0xE5E7EB)
            })
            .cursor_text()
            .track_focus(&self.focus_handle)
            .on_click(cx.listener(|this, _evt, window, _cx| {
                this.is_focused = true;
                window.focus(&this.focus_handle);
            }))
            .on_key_down(cx.listener(|this, evt: &KeyDownEvent, _window, cx| {
                match evt.keystroke.key.as_str() {
                    "backspace" => {
                        this.handle_backspace(cx);
                    }
                    "escape" => {
                        this.clear_search(cx);
                        this.is_focused = false;
                        cx.notify();
                    }
                    _ => {
                        // 处理普通字符输入
                        if let Some(key_char) = &evt.keystroke.key_char {
                            this.handle_key_input(key_char, cx);
                        }
                    }
                }
            }))
            // 搜索图标
            .child(
                svg()
                    .path("search.svg")
                    .size_4()
                    .text_color(rgb(0x9CA3AF))
                    .mr_2()
                    .flex_shrink_0(),
            )
            // 搜索文本显示
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .items_center()
                    .overflow_hidden()
                    .text_sm()
                    .when(is_placeholder, |this| this.text_color(rgb(0x9CA3AF)))
                    .when(!is_placeholder, |this| this.text_color(rgb(0x374151)))
                    .child(display_text),
            )
            // 清除按钮
            .map(|el| {
                if has_query {
                    el.child(
                        div()
                            .id("clear-search")
                            .size(px(20.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .cursor_pointer()
                            .ml_2()
                            .hover(|s| s.bg(rgb(0xE5E7EB)))
                            .child(svg().path("close.svg").size_3().text_color(rgb(0x9CA3AF)))
                            .on_click(cx.listener(|this, _evt, _window, cx| {
                                this.clear_search(cx);
                            })),
                    )
                } else {
                    el
                }
            })
    }
}

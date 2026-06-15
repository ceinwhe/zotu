use gpui::{prelude::FluentBuilder, *};

use crate::{
    application::{AppAction, AppController, AppEvent},
    theme::*,
};

pub struct SearchBox {
    controller: Entity<AppController>,
    focus_handle: FocusHandle,
}

impl SearchBox {
    pub fn new(controller: Entity<AppController>, cx: &mut Context<Self>) -> Self {
        cx.subscribe(&controller, |_this, _controller, event, cx| {
            if matches!(event, AppEvent::SearchChanged | AppEvent::NavigationChanged) {
                cx.notify();
            }
        })
        .detach();

        Self {
            controller,
            focus_handle: cx.focus_handle(),
        }
    }

    fn set_query(&self, query: String, cx: &mut Context<Self>) {
        let action = if query.is_empty() {
            AppAction::SearchCleared
        } else {
            AppAction::SearchChanged(query)
        };
        self.controller.update(cx, |controller, cx| {
            controller.dispatch(action, cx);
        });
    }
}

impl Render for SearchBox {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let query = self.controller.read(cx).state().search_query().to_string();
        let has_query = !query.is_empty();
        let display_text = if has_query {
            query.clone()
        } else {
            "搜索歌曲、歌手、专辑...".to_string()
        };
        let is_focused = self.focus_handle.is_focused(window);

        div()
            .id("search-container")
            .key_context("SearchInput")
            .w(px(SEARCH_BOX_WIDTH))
            .h(px(SEARCH_BOX_HEIGHT))
            .flex()
            .flex_row()
            .items_center()
            .px_3()
            .bg(bg_input())
            .rounded_lg()
            .border_1()
            .border_color(input_focus_ring(is_focused))
            .cursor_text()
            .track_focus(&self.focus_handle)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _evt, window, _cx| {
                    window.focus(&this.focus_handle);
                }),
            )
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                let mut query = this.controller.read(cx).state().search_query().to_string();
                match event.keystroke.key.as_str() {
                    "backspace" => {
                        query.pop();
                        this.set_query(query, cx);
                    }
                    "escape" => this.set_query(String::new(), cx),
                    _ => {
                        if let Some(character) = &event.keystroke.key_char {
                            query.push_str(character);
                            this.set_query(query, cx);
                        }
                    }
                }
            }))
            .child(
                svg()
                    .path("svg/search.svg")
                    .size_4()
                    .text_color(text_placeholder())
                    .mr_2()
                    .flex_shrink_0(),
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .items_center()
                    .overflow_hidden()
                    .text_sm()
                    .when(!has_query, |this| this.text_color(text_placeholder()))
                    .when(has_query, |this| this.text_color(text_secondary()))
                    .child(display_text),
            )
            .when(has_query, |element| {
                element.child(
                    div()
                        .id("clear-search")
                        .size(px(20.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .cursor_pointer()
                        .ml_2()
                        .hover(|style| style.bg(bg_active()))
                        .child(
                            svg()
                                .path("svg/close.svg")
                                .size_3()
                                .text_color(text_placeholder()),
                        )
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _evt, _window, cx| {
                                this.set_query(String::new(), cx);
                            }),
                        ),
                )
            })
    }
}

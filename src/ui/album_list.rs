use std::sync::Arc;

use gpui::{prelude::FluentBuilder, *};
use rfd::AsyncFileDialog;

use crate::{
    application::{AppAction, AppController, AppEvent, AppRoute, LibraryAction, PlaybackAction},
    theme::*,
};

use super::{ContextMenu, format_duration};

pub struct AlbumList {
    controller: Entity<AppController>,
    context_menu: Entity<ContextMenu>,
}

impl AlbumList {
    pub fn new(controller: Entity<AppController>, cx: &mut Context<Self>) -> Self {
        cx.subscribe(&controller, |_this, _controller, event, cx| {
            if matches!(
                event,
                AppEvent::LibraryChanged | AppEvent::NavigationChanged | AppEvent::SearchChanged
            ) {
                cx.notify();
            }
        })
        .detach();

        Self {
            context_menu: cx.new(|_| ContextMenu::new(controller.clone())),
            controller,
        }
    }

    fn pick_music_folder(&self, cx: &mut Context<Self>) {
        let controller = self.controller.clone();
        cx.spawn(
            async move |_this: WeakEntity<AlbumList>, cx: &mut AsyncApp| {
                let folder = AsyncFileDialog::new()
                    .set_title("选择音乐文件夹")
                    .pick_folder()
                    .await;

                if let Some(folder) = folder {
                    let path = folder.path().to_string_lossy().to_string();
                    let _ = controller.update(cx, |controller, cx| {
                        controller
                            .dispatch(AppAction::Library(LibraryAction::ScanDirectory(path)), cx);
                    });
                }
            },
        )
        .detach();
    }
}

impl Render for AlbumList {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.controller.read(cx).state();
        let items = state.visible_tracks();
        let is_searching = state.is_searching();
        let search_query = state.search_query().to_string();
        let route = state.route();
        let controller = self.controller.clone();
        let context_menu = self.context_menu.clone();

        div()
            .id("album-list-container")
            .w_full()
            .flex_1()
            .min_h_0()
            .relative()
            .child(
                div()
                    .id("album-list")
                    .size_full()
                    .when(items.is_empty() && is_searching, |this| {
                        this.size_full()
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .gap_4()
                            .child(
                                svg()
                                    .path("svg/search.svg")
                                    .size(Pixels::from(64.0))
                                    .text_color(text_muted()),
                            )
                            .child(
                                div()
                                    .text_lg()
                                    .text_color(text_tertiary())
                                    .child(format!("未找到 \"{}\" 相关结果", search_query)),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(text_placeholder())
                                    .child("请尝试其他关键词"),
                            )
                    })
                    .when(
                        items.is_empty() && route == AppRoute::Library && !is_searching,
                        |this| {
                            this.size_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    div()
                                        .id("no-songs-message")
                                        .h(Pixels::from(40.0))
                                        .w(Pixels::from(80.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child("添加歌曲")
                                        .bg(accent_blue())
                                        .rounded_lg()
                                        .cursor_pointer()
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(|this, _event, _window, cx| {
                                                this.pick_music_folder(cx);
                                            }),
                                        ),
                                )
                        },
                    )
                    .when(!items.is_empty(), move |this| {
                        let controller = controller.clone();
                        let context_menu = context_menu.clone();
                        this.size_full()
                            .flex()
                            .flex_col()
                            .overflow_y_scroll()
                            .children(items.iter().enumerate().map(|(index, item)| {
                                let item = item.clone();
                                let item_id = item.id();
                                div()
                                    .id(ElementId::Name(format!("song-{index}").into()))
                                    .w_full()
                                    .flex()
                                    .flex_row()
                                    .gap_4()
                                    .px_4()
                                    .py_2()
                                    .border_b_1()
                                    .border_color(border_default())
                                    .bg(bg_card())
                                    .hover(|style| style.bg(bg_hover()))
                                    .cursor_pointer()
                                    .child(
                                        div()
                                            .flex_shrink_0()
                                            .size(Pixels::from(COVER_THUMB_SIZE))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .when_some(item.cover_64(), |this, cover| {
                                                this.child(
                                                    img(Arc::new(Image::from_bytes(
                                                        ImageFormat::Jpeg,
                                                        cover.to_vec(),
                                                    )))
                                                    .size_full(),
                                                )
                                            })
                                            .when_none(&item.cover_64(), |this| {
                                                this.child(
                                                    svg()
                                                        .path("svg/album.svg")
                                                        .size_full()
                                                        .text_color(text_muted()),
                                                )
                                            }),
                                    )
                                    .child(
                                        div()
                                            .flex_basis(DefiniteLength::Fraction(0.4))
                                            .flex()
                                            .flex_col()
                                            .items_start()
                                            .text_left()
                                            .truncate()
                                            .text_ellipsis()
                                            .child(
                                                div()
                                                    .text_base()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .truncate()
                                                    .child(item.title()),
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .font_weight(FontWeight::LIGHT)
                                                    .truncate()
                                                    .child(item.artist()),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex_basis(DefiniteLength::Fraction(0.3))
                                            .flex()
                                            .items_center()
                                            .text_left()
                                            .truncate()
                                            .text_ellipsis()
                                            .text_sm()
                                            .font_weight(FontWeight::LIGHT)
                                            .child(item.album()),
                                    )
                                    .child(
                                        div()
                                            .flex_basis(DefiniteLength::Fraction(0.2))
                                            .flex()
                                            .items_center()
                                            .justify_end()
                                            .text_sm()
                                            .font_weight(FontWeight::LIGHT)
                                            .child(format_duration(item.duration())),
                                    )
                                    .on_mouse_down(MouseButton::Left, {
                                        let controller = controller.clone();
                                        cx.listener(move |_this, _event, _window, cx| {
                                            controller.update(cx, |controller, cx| {
                                                controller.dispatch(
                                                    AppAction::Playback(PlaybackAction::PlayTrack(
                                                        item_id,
                                                    )),
                                                    cx,
                                                );
                                            });
                                        })
                                    })
                                    .on_mouse_down(MouseButton::Right, {
                                        let controller = controller.clone();
                                        let context_menu = context_menu.clone();
                                        cx.listener(
                                            move |_this, event: &MouseDownEvent, _window, cx| {
                                                let is_favorite = controller
                                                    .read(cx)
                                                    .state()
                                                    .is_favorite(&item_id);
                                                context_menu.update(cx, |menu, cx| {
                                                    menu.show(
                                                        item_id,
                                                        event.position,
                                                        is_favorite,
                                                        cx,
                                                    );
                                                });
                                            },
                                        )
                                    })
                            }))
                    }),
            )
            .child(self.context_menu.clone())
    }
}

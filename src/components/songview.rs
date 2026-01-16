use gpui::DefiniteLength;
use gpui::*;
use rfd::AsyncFileDialog;
use std::sync::Arc;

use crate::db::metadata::AlbumInfo;
use crate::play::player::Player;
use crate::util::format_duration;

#[derive(Clone)]
pub struct AlbumList {
    items: Option<Arc<Vec<AlbumInfo>>>,
}

impl AlbumList {
    pub fn new(items: Option<Arc<Vec<AlbumInfo>>>) -> Self {
        Self {
            items,
        }
    }

    pub fn items(&self) -> Option<Arc<Vec<AlbumInfo>>> {
        self.items.as_ref().map(Arc::clone)
    }
}

impl Render for AlbumList {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        match &self.items {
            None => div()
                .id("song-list")
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .id("no-songs-message")
                        .text_center()
                        .child("添加歌曲")
                        .hover(|style| style.bg(rgb(0xEEEEEE)))
                        .rounded_lg()
                        .cursor_pointer()
                        .on_click(cx.listener(|_this, _event, _window, cx| {
                            cx.spawn(
                                async move |_this: WeakEntity<AlbumList>, _cx: &mut AsyncApp| {
                                    let _files = AsyncFileDialog::new()
                                        .set_title("选择音乐文件")
                                        .pick_folder()
                                        .await;
                                },
                            )
                            .detach();
                        })),
                ),
            Some(items) => {
                let items = Arc::clone(items);
                div()
                    .id("song-list")
                    .size_full()
                    .flex()
                    .flex_col()
                    .overflow_y_scroll()
                    .children(items.iter().enumerate().map(|(idx, item)| {
                        let item_id = item.id;
                        let items_for_click = Arc::clone(&items);

                        div()
                            .id(ElementId::Name(format!("song-{}", idx).into()))
                            .w_full()
                            .flex()
                            .flex_row()
                            .gap_4()
                            .px_4()
                            .py_2()
                            .border_b_1()
                            .bg(rgb(0xFAFAF9))
                            .hover(move |style| style.bg(rgb(0xEEEEEE)))
                            .cursor_pointer()
                            .child(
                                div()
                                    .flex_basis(DefiniteLength::Fraction(0.5))
                                    .flex()
                                    .flex_col()
                                    .items_start()
                                    .text_left()
                                    .truncate()
                                    .text_ellipsis()
                                    .child(item.title())
                                    .child(item.artist()),
                            )
                            .child(
                                div()
                                    .flex_basis(DefiniteLength::Fraction(0.3))
                                    .flex()
                                    .items_center()
                                    .text_left()
                                    .truncate()
                                    .text_ellipsis()
                                    .child(item.album()),
                            )
                            .child(
                                div()
                                    .flex_basis(DefiniteLength::Fraction(0.2))
                                    .flex()
                                    .items_center()
                                    .justify_end()
                                    .child(format!("{}", format_duration(item.duration()))),
                            )
                            .on_click(cx.listener(move |_this, _event, _window, cx| {
                                // 点击时设置播放列表并播放这首歌
                                cx.update_global::<Player, _>(|player, _cx| {
                                    // 如果还没设置播放列表，先设置
                                    if !player.has_playlist() {
                                        player.set_playlist(Arc::clone(&items_for_click));
                                    }
                                    // 查找并播放点击的歌曲
                                    if let Some(item) =
                                        items_for_click.iter().find(|i| i.id == item_id)
                                    {
                                        player.play_track(item);
                                    }
                                });
                            }))
                    }))
            }
        }

        // div()
        //     .id("song-list")
        //     .size_full()
        //     .flex()
        //     .flex_col()
        //     .overflow_y_scroll()
        //     .children(items.iter().enumerate().map(|(idx, item)| {
        //         let item_id = item.id;
        //         let items_for_click = Arc::clone(&self.items);

        //         div()
        //             .id(ElementId::Name(format!("song-{}", idx).into()))
        //             .w_full()
        //             .flex()
        //             .flex_row()
        //             .gap_4()
        //             .px_4()
        //             .py_2()
        //             .border_b_1()
        //             .bg(rgb(0xFAFAF9))
        //             .hover(move |style| style.bg(rgb(0xEEEEEE)))
        //             .cursor_pointer()
        //             .child(
        //                 div()
        //                     .flex_basis(DefiniteLength::Fraction(0.5))
        //                     .flex()
        //                     .flex_col()
        //                     .items_start()
        //                     .text_left()
        //                     .truncate()
        //                     .text_ellipsis()
        //                     .child(item.title())
        //                     .child(item.artist()),
        //             )
        //             .child(
        //                 div()
        //                     .flex_basis(DefiniteLength::Fraction(0.3))
        //                     .flex()
        //                     .items_center()
        //                     .text_left()
        //                     .truncate()
        //                     .text_ellipsis()
        //                     .child(item.album()),
        //             )
        //             .child(
        //                 div()
        //                     .flex_basis(DefiniteLength::Fraction(0.2))
        //                     .flex()
        //                     .items_center()
        //                     .justify_end()
        //                     .child(format!("{}", format_duration(item.duration()))),
        //             )
        //             .on_click(cx.listener(move |_this, _event, _window, cx| {
        //                 // 点击时设置播放列表并播放这首歌
        //                 cx.update_global::<Player, _>(|player, _cx| {
        //                     // 如果还没设置播放列表，先设置
        //                     if !player.has_playlist() {
        //                         player.set_playlist(Arc::clone(&items_for_click));
        //                     }
        //                     // 查找并播放点击的歌曲
        //                     if let Some(item) = items_for_click.iter().find(|i| i.id == item_id) {
        //                         player.play_track(item);
        //                     }
        //                 });
        //             }))
        //     }))
    }
}

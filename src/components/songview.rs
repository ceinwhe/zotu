use gpui::DefiniteLength;
use gpui::prelude::FluentBuilder;
use gpui::*;
use rfd::AsyncFileDialog;
use std::sync::Arc;

use crate::db::db::DB;
use crate::db::metadata::AlbumInfo;
use crate::play::player::Player;
use crate::util::format_duration;

#[derive(Clone)]
pub struct AlbumList {
    items: Option<Arc<Vec<AlbumInfo>>>,
    _pick_folder_task: Option<Arc<Task<()>>>,
}

impl AlbumList {
    pub fn new(items: Option<Arc<Vec<AlbumInfo>>>) -> Self {
        Self {
            items,
            _pick_folder_task: None,
        }
    }

    pub fn items(&self) -> Option<Arc<Vec<AlbumInfo>>> {
        self.items.as_ref().map(Arc::clone)
    }

    pub fn refresh_items(&mut self, cx: &mut Context<Self>) {
        let items = cx.global::<DB>().load_all_albums().map(Arc::new);
        self.items = items;
        cx.notify();
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
                        .on_click(cx.listener(|this, _event, _window, cx| {
                            // 使用异步的 FileDialog
                            let task = cx.spawn(
                                async move |this: WeakEntity<AlbumList>, cx: &mut AsyncApp| {
                                    let folder = AsyncFileDialog::new()
                                        .set_title("选择音乐文件夹")
                                        .pick_folder()
                                        .await;

                                    if let Some(folder) = folder {
                                        let path = folder.path().to_string_lossy().to_string();
                                        let _ = cx.update(|cx: &mut App| {
                                            cx.update_global::<DB, _>(|db, _cx| {
                                                let _ = db.add_metadata_to_library(&path);
                                            });
                                        });

                                        // 添加完成后，更新歌曲列表
                                        let _ = this.update(cx, |this, cx| {
                                            this.refresh_items(cx);
                                        });
                                    }
                                },
                            );
                            this._pick_folder_task = Some(Arc::new(task));
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
                    .scrollbar_width(Pixels::from(10.0))
                    .children(items.iter().enumerate().map(|(idx, item)| {
                        let item_id = item.id();
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
                            //专辑封面
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .size(Pixels::from(48.0))
                                    .flex()
                                    .content_center()
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
                                            svg().path("album.svg").size_full().text_color(black()),
                                        )
                                    }),
                            )
                            //歌曲名字和歌手
                            .child(
                                div()
                                    .flex_basis(DefiniteLength::Fraction(0.4))
                                    .flex()
                                    .flex_col()
                                    .items_start()
                                    .text_left()
                                    .truncate()
                                    .text_ellipsis()
                                    .child(item.title())
                                    .child(item.artist()),
                            )
                            //专辑名字
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
                            //时长
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
                                        items_for_click.iter().find(|i| i.id() == item_id)
                                    {
                                        player.play_track(item);
                                    }
                                });
                            }))
                    }))
            }
        }
    }
}

use gpui::DefiniteLength;
use gpui::prelude::FluentBuilder;
use gpui::*;
use rfd::AsyncFileDialog;
use std::sync::Arc;

use crate::db::library_state::LibraryState;
use crate::db::{db::DB, metadata::AlbumInfo, table::Table};
use crate::play::player::Player;
use crate::util::format_duration;

/// 当前显示的视图类型
#[derive(Clone, Copy, PartialEq, Default)]
pub enum ViewType {
    #[default]
    Library,
    Favorite,
    History,
}

#[derive(Clone)]
pub struct AlbumList {
    /// 当前显示的视图类型
    view_type: ViewType,
    /// LibraryState 引用，作为唯一数据源
    library_state: Entity<LibraryState>,
    _pick_folder_task: Option<Arc<Task<()>>>,
}

impl AlbumList {
    pub fn new(library_state: Entity<LibraryState>) -> Self {
        Self {
            view_type: ViewType::Library,
            library_state,
            _pick_folder_task: None,
        }
    }

    /// 切换视图类型，并返回当前列表用于更新播放列表
    pub fn set_view_type(
        &mut self,
        view_type: ViewType,
        cx: &Context<Self>,
    ) -> Option<Arc<Vec<AlbumInfo>>> {
        self.view_type = view_type;
        self.get_current_items(cx)
    }

    /// 获取当前视图类型
    pub fn view_type(&self) -> ViewType {
        self.view_type
    }

    /// 获取 LibraryState 引用
    pub fn library_state(&self) -> &Entity<LibraryState> {
        &self.library_state
    }

    /// 获取当前显示的列表（根据视图类型从 LibraryState 读取）
    fn get_current_items(&self, cx: &Context<Self>) -> Option<Arc<Vec<AlbumInfo>>> {
        let state = self.library_state.read(cx);
        match self.view_type {
            ViewType::Library => state.library(),
            ViewType::Favorite => state.favorites(),
            ViewType::History => state.history(),
        }
    }

    /// 刷新曲库（从数据库重新加载）
    pub fn refresh_library(&self, cx: &mut Context<Self>) {
        let items = cx.global::<DB>().load_all_albums().unwrap_or_default();
        self.library_state.update(cx, |state, cx| {
            state.update_library(items, cx);
        });
        cx.notify();
    }
}

impl Render for AlbumList {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let items = self.get_current_items(cx);

        match items {
            None => {
                // 空列表视图
                div()
                    .id("song-list")
                    .size_full()
                    .flex()
                    .content_center()
                    .justify_center()
                    .child(
                        div()
                            .id("no-songs-message")
                            .h(Pixels::from(20.0))
                            .w(Pixels::from(60.0))
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

                                            // 添加完成后，更新曲库
                                            let _ = this.update(cx, |this, cx| {
                                                this.refresh_library(cx);
                                            });
                                        }
                                    },
                                );
                                this._pick_folder_task = Some(Arc::new(task));
                            })),
                    )
            }
            Some(items) => {
                // 歌曲列表视图
                div()
                    .id("song-list")
                    .size_full()
                    .flex()
                    .flex_col()
                    .overflow_y_scroll()
                    .children(items.iter().enumerate().map(|(idx, item)| {
                        // 克隆 item 以获得所有权（Arc 克隆成本低）
                        let item = item.clone();
                        let item_id_l = item.id();
                        let item_id_r = Arc::clone(&item_id_l);

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
                            // 专辑封面
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
                            // 歌曲名字和歌手
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
                            // 专辑名字
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
                            // 时长
                            .child(
                                div()
                                    .flex_basis(DefiniteLength::Fraction(0.2))
                                    .flex()
                                    .items_center()
                                    .justify_end()
                                    .child(format!("{}", format_duration(item.duration()))),
                            )
                            .on_click(cx.listener(move |this, _event, _window, cx| {
                                // 通过 LibraryState 添加到历史
                                this.library_state.update(cx, |state, cx| {
                                    state.add_to_history(&item_id_l, cx);
                                });
                                // 同时写入数据库
                                let _ = cx
                                    .global::<DB>()
                                    .add_to_table(Table::History, &item_id_l);

                                cx.update_global::<Player, _>(|player, _cx| {
                                    // 播放点击的歌曲
                                    player.play_track(&item);
                                });
                            }))
                            .on_mouse_down(
                                MouseButton::Right,
                                cx.listener(move|this, _evt, _window, cx| {
                                    // 通过 LibraryState 添加到收藏
                                    this.library_state.update(cx, |state, cx| {
                                        state.add_to_favorites(&item_id_r, cx);
                                    });
                                    // 同时写入数据库
                                    let _ = cx.global::<DB>().add_to_table(
                                        Table::Favorite,
                                        &item_id_r,
                                    );
                                }),
                            )
                    }))
            }
        }
    }
}

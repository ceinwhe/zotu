use gpui::{prelude::FluentBuilder, *};
use rfd::AsyncFileDialog;
use std::sync::Arc;

use crate::{
    db::{db::DB, dbstate::LibraryState, metadata::AlbumInfo, table::Table},
    play::player::Player,
    ui::menu::{MenuContext, MenuAction},
    util::format_duration,
};

/// 当前显示的视图类型
#[derive(Clone, Copy, PartialEq, Default)]
pub enum ViewType {
    #[default]
    Library,
    Favorite,
    History,
    Search,
}

#[derive(Clone)]
pub struct AlbumList {
    /// 当前显示的视图类型
    view_type: ViewType,
    /// LibraryState 引用，作为唯一数据源
    library_state: Entity<LibraryState>,
    /// 搜索关键词
    search_query: String,
    /// 搜索结果
    search_results: Arc<Vec<AlbumInfo>>,
    /// 右键菜单实体
    context_menu: Entity<MenuContext>,
}

impl AlbumList {
    pub fn new(library_state: Entity<LibraryState>, cx: &mut Context<Self>) -> Self {
        let context_menu = cx.new(|_| MenuContext::new());

        cx.subscribe(&context_menu, |this,_that,evt: &MenuAction,cx|{
            match evt {
                MenuAction::AddToFavorite(album_id) => {
                    // 添加到收藏
                    this.library_state.update(cx, |state, cx| {
                        state.add_to_favorites(album_id, cx);
                    });
                }
                MenuAction::RemoveFromFavorite(album_id) => {
                    // 从收藏移除
                    this.library_state.update(cx, |state, cx| {
                        state.remove_from_favorites(album_id, cx);
                    });
                }
            }
        }).detach();

        Self {
            view_type: ViewType::Library,
            library_state,
            search_query: String::new(),
            search_results: Arc::new(Vec::new()),
            context_menu,
        }
    }

    /// 切换视图类型，并返回当前列表用于更新播放列表
    pub fn set_view_type(
        &mut self,
        view_type: ViewType,
        cx: &Context<Self>,
    ) -> Arc<Vec<AlbumInfo>> {
        self.view_type = view_type;
        // 切换视图时清除搜索
        if view_type != ViewType::Search {
            self.search_query.clear();
            self.search_results = Arc::new(Vec::new());
        }
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

    /// 执行搜索
    pub fn search(&mut self, query: &str, cx: &mut Context<Self>) -> Arc<Vec<AlbumInfo>> {
        self.search_query = query.to_lowercase();
        self.view_type = ViewType::Search;

        // 在全部曲库中搜索
        let library = self.library_state.read(cx).library();
        let results: Vec<AlbumInfo> = library
            .iter()
            .filter(|item| {
                let query = &self.search_query;
                // 搜索歌曲名、歌手、专辑
                item.title().to_lowercase().contains(query)
                    || item.artist().to_lowercase().contains(query)
                    || item.album().to_lowercase().contains(query)
            })
            .cloned()
            .collect();

        self.search_results = Arc::new(results);
        cx.notify();
        Arc::clone(&self.search_results)
    }

    /// 清除搜索
    pub fn clear_search(&mut self, cx: &mut Context<Self>) {
        self.search_query.clear();
        self.search_results = Arc::new(Vec::new());
        self.view_type = ViewType::Library;
        cx.notify();
    }

    /// 获取当前显示的列表（根据视图类型从 LibraryState 读取）
    fn get_current_items(&self, cx: &Context<Self>) -> Arc<Vec<AlbumInfo>> {
        let state = self.library_state.read(cx);
        match self.view_type {
            ViewType::Library => state.library(),
            ViewType::Favorite => state.favorites(),
            ViewType::History => state.history(),
            ViewType::Search => Arc::clone(&self.search_results),
        }
    }

    /// 刷新曲库（从数据库重新加载）
    pub fn refresh_library(&self, cx: &mut Context<Self>) {
        let items = cx.global::<DB>().load_all_albums();
        self.library_state.update(cx, |state, cx| {
            state.update_library(items, cx);
        });
        cx.notify();
    }
}

impl Render for AlbumList {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let items = self.get_current_items(cx);
        let is_search = self.view_type == ViewType::Search;
        let search_query = self.search_query.clone();

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
                    // 搜索无结果提示
                    .when(items.is_empty() && is_search, |this| {
                        this.size_full()
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .gap_4()
                            .child(
                                svg()
                                    .path("search.svg")
                                    .size(Pixels::from(64.0))
                                    .text_color(rgb(0xD1D5DB)),
                            )
                            .child(
                                div()
                                    .text_lg()
                                    .text_color(rgb(0x6B7280))
                                    .child(format!("未找到 \"{}\" 相关结果", search_query)),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x9CA3AF))
                                    .child("请尝试其他关键词"),
                            )
                    })
                    .when(
                        items.is_empty() && self.view_type == ViewType::Library,
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
                                        .bg(rgb(0xAED6F1))
                                        .rounded_lg()
                                        .cursor_pointer()
                                        .on_click(cx.listener(|_this, _event, _window, cx| {
                                            // 使用异步的 FileDialog
                                            cx.spawn(
                                    async move |this: WeakEntity<AlbumList>, cx: &mut AsyncApp| {
                                        let folder = AsyncFileDialog::new()
                                            .set_title("选择音乐文件夹")
                                            .pick_folder()
                                            .await;

                                        if let Some(folder) = folder {
                                            let path = folder.path().to_string_lossy().to_string();
                                            cx.update(|cx: &mut App| {
                                                cx.update_global::<DB, _>(|db, _cx| {
                                                    db.add_metadata_to_library(&path).unwrap();
                                                });
                                            }).unwrap();

                                            // 添加完成后，更新曲库
                                            this.update(cx, |this, cx| {
                                                this.refresh_library(cx);
                                            }).unwrap();
                                        }
                                    },
                                )
                                .detach();
                                        })),
                                )
                        },
                    )
                    .when(!items.is_empty(), move |this| {
                        this.size_full()
                            .flex()
                            .flex_col()
                            .overflow_y_scroll()
                            .children(items.iter().enumerate().map(|(idx, item)| {
                                // 克隆 item 以获得所有权（Arc 克隆成本低）
                                let item = item.clone();
                                let item_id = item.id();
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
                                                    svg()
                                                        .path("album.svg")
                                                        .size_full()
                                                        .text_color(black()),
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
                                            .text_sm()
                                            .font_weight(FontWeight::LIGHT)
                                            .child(item.album()),
                                    )
                                    // 时长
                                    .child(
                                        div()
                                            .flex_basis(DefiniteLength::Fraction(0.2))
                                            .flex()
                                            .items_center()
                                            .justify_end()
                                            .text_sm()
                                            .font_weight(FontWeight::LIGHT)
                                            .child(format!("{}", format_duration(item.duration()))),
                                    )
                                    .on_click({
                                        let item_for_play = item.clone();
                                        cx.listener(move |this, _event, _window, cx| {
                                            // 通过 LibraryState 添加到历史
                                            this.library_state.update(cx, |state, cx| {
                                                state.add_to_history(&item.id(), cx);
                                            });
                                            // 同时写入数据库
                                            cx.global::<DB>()
                                                .add_to_table(Table::History, &item.id())
                                                .unwrap();

                                            cx.update_global::<Player, _>(|player, _cx| {
                                                // 播放点击的歌曲
                                                player.play_track(&item_for_play);
                                            });
                                        })
                                    })
                                    .on_mouse_down(MouseButton::Right, {
                                        cx.listener(
                                            move |this, evt: &MouseDownEvent, _window, cx| {
                                                // 显示右键菜单
                                                this.context_menu.update(cx, move |menu, cx| {
                                                    menu.show(&item_id, cx, evt.position);
                                                });
                                            },
                                        )
                                    })
                            }))
                    }),
            )
    }
}

use gpui::prelude::*;
use gpui::*;

use crate::components::playbar::PlayBar;
use crate::components::setting::Setting;
use crate::components::sidebar::{Sidebar, SidebarMessage};
use crate::components::songview::{AlbumList, ViewType};
use crate::components::titlebar::TitleBar;
use crate::db::library_state::LibraryState;
use crate::db::{db::DB, table::Table};
use crate::play::player::Player;

enum ViewShow {
    SongView,
    Setting,
}

// 主应用结构
pub struct Zotu {
    view_show: ViewShow,
    song_view: Entity<AlbumList>,
    setting: Entity<Setting>,
    play_bar: Entity<PlayBar>,
    title_bar: Entity<TitleBar>,
    sidebar: Entity<Sidebar>,
}

impl Zotu {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        // 从数据库加载初始数据
        let library_list = cx.global::<DB>().load_all_albums();
        let favorite_uuid_list = cx.global::<DB>().get_all_uuids(Table::Favorite);
        let history_uuid_list = cx.global::<DB>().get_all_uuids(Table::History);

        // 创建 LibraryState Entity - 作为唯一的数据源
        let library_state =
            cx.new(|_cx| LibraryState::new(library_list, favorite_uuid_list, history_uuid_list));

        // 创建歌曲列表视图，持有 LibraryState
        let song_view = cx.new(|_cx| AlbumList::new(library_state));

        let play_bar = cx.new(|cx| PlayBar::new(cx));
        let title_bar = cx.new(|_| TitleBar);
        let sidebar = cx.new(|_| Sidebar);
        let setting = cx.new(|_| Setting);

        // 订阅侧边栏消息 - 通过 song_view 来访问 library_state
        cx.subscribe(&sidebar, |this, _that, evt, cx| match evt {
            SidebarMessage::Settings => {
                this.view_show = ViewShow::Setting;
                cx.notify();
            }
            SidebarMessage::Library => {
                this.view_show = ViewShow::SongView;
                let list = this
                    .song_view
                    .update(cx, |view, cx| view.set_view_type(ViewType::Library, cx));
                if let Some(list) = list {
                    cx.global_mut::<Player>().set_playlist(list);
                }
                cx.notify();
            }
            SidebarMessage::Favorite => {
                this.view_show = ViewShow::SongView;
                let list = this
                    .song_view
                    .update(cx, |view, cx| view.set_view_type(ViewType::Favorite, cx));
                if let Some(list) = list {
                    cx.global_mut::<Player>().set_playlist(list);
                }
                cx.notify();
            }
            SidebarMessage::History => {
                this.view_show = ViewShow::SongView;
                let list = this
                    .song_view
                    .update(cx, |view, cx| view.set_view_type(ViewType::History, cx));
                if let Some(list) = list {
                    cx.global_mut::<Player>().set_playlist(list);
                }
                cx.notify();
            }
        })
        .detach();

        Self {
            view_show: ViewShow::SongView,
            song_view,
            setting,
            play_bar,
            title_bar,
            sidebar,
        }
    }
}

impl Render for Zotu {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_row()
            .bg(rgb(0xFAFAFA))
            // 侧边栏
            .child(self.sidebar.clone())
            .child(
                div()
                    .h_full()
                    .w_full()
                    .flex()
                    .flex_col()
                    .child(self.title_bar.clone())
                    .map(|parent| match self.view_show {
                        ViewShow::Setting => parent.child(self.setting.clone()),
                        ViewShow::SongView => parent
                            .child(self.song_view.clone())
                            .child(self.play_bar.clone()),
                    }),
            )
    }
}

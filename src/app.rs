use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;

use crate::components::playbar::PlayBar;
use crate::components::setting::Setting;
use crate::components::sidebar::{Sidebar, SidebarMessage};
use crate::components::songview::AlbumList;
use crate::components::titlebar::TitleBar;
use crate::db::db::DB;
// use crate::play::player::Player;

enum ViewShow {
    Song,
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
        let lists = cx.global::<DB>().load_all_albums().map(Arc::new);

        // 创建歌曲列表视图
        let song_view = cx.new(|_cx| AlbumList::new(lists));
        let play_bar = cx.new(|cx| PlayBar::new(cx));
        let title_bar = cx.new(|_| TitleBar);
        let sidebar = cx.new(|_| Sidebar);
        let setting = cx.new(|_| Setting);
        // 初始化播放器的播放列表?
        // cx.update_global::<Player, _>(|player, _cx| {
        //     player.set_playlist(Arc::clone(&items));
        // });

        cx.subscribe(&sidebar, |this, _that, evt, cx| match evt {
            SidebarMessage::Settings => {
                this.view_show = ViewShow::Setting;
                cx.notify();
            }
            SidebarMessage::Library => {
                this.view_show = ViewShow::Song;
                cx.notify();
            }
            _ => {}
        })
        .detach();

        Self {
            view_show: ViewShow::Song,
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
            //侧边栏
            .child(self.sidebar.clone())
            .child(
                div()
                    .h_full()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(self.title_bar.clone())
                    .map(|parent| match self.view_show {
                        ViewShow::Setting => parent.child(self.setting.clone()),
                        ViewShow::Song => parent
                            .child(self.song_view.clone())
                            .child(self.play_bar.clone()),
                    }),
            )
    }
}

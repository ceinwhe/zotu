use gpui::prelude::*;
use gpui::*;

use crate::components::playbar::PlayBar;
use crate::components::sidebar::Sidebar;
use crate::components::songview::AlbumList;
use crate::components::titlebar::TitleBar;
use crate::play::metadata::AlbumInfo;
use crate::play::player::Player;
use std::sync::Arc;

// 主应用结构
pub struct Zotu {
    pub song_view: Entity<AlbumList>,
    pub play_bar: Entity<PlayBar>,
}

impl Zotu {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>, list: Vec<AlbumInfo>) -> Self {
        let items = Arc::new(list);

        // 创建歌曲列表视图
        let song_view = cx.new(|_cx| AlbumList::new((*items).clone()));

        // 初始化播放器的播放列表
        cx.update_global::<Player, _>(|player, _cx| {
            player.set_playlist(Arc::clone(&items));
        });

        // 创建播放栏
        let play_bar = cx.new(|cx| PlayBar::new(cx));

        Self {
            song_view,
            play_bar,
        }
    }
}
impl Render for Zotu {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_row()
            .bg(rgb(0xFAFAFA))
            //侧边栏
            .child(cx.new(|_cx| Sidebar))
            .child(
                div()
                    .h_full()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(cx.new(|_cx| TitleBar))
                    .child(self.song_view.clone())
                    .child(self.play_bar.clone()),
            )
    }
}

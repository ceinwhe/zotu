use gpui::{prelude::FluentBuilder,*};

use crate::{
    components::{
        playbar::{PlayBar,PlayBarMessage},
        now_playing::{PlayerDetail},
        setting::Setting,
        sidebar::{SideBar, SidebarItem},
        songview::{AlbumList, ViewType},
        titlebar::TitleBar,
    },
    db::{db::DB, dbstate::LibraryState, table::Table},
    play::player::Player,
    ui::search::{ClearSearchEvent, SearchEvent},
};

// 主应用结构
pub struct Zotu {
    view_type: SidebarItem,
    song_view: Entity<AlbumList>,
    setting: Entity<Setting>,
    play_bar: Entity<PlayBar>,
    title_bar: Entity<TitleBar>,
    sidebar: Entity<SideBar>,
    now_playing: Entity<PlayerDetail>,
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
        let song_view = cx.new(|cx| AlbumList::new(library_state, cx));

        let play_bar = cx.new(|cx| PlayBar::new(cx));
        let title_bar = cx.new(|cx| TitleBar::new(cx));
        let sidebar = cx.new(|_| SideBar::new());
        let setting = cx.new(|_| Setting);
        let player_detail = cx.new(|_| PlayerDetail::new());

        
        // 订阅标题栏搜索事件
        cx.subscribe(&title_bar, |this, _that, evt: &SearchEvent, cx| {
            this.view_type = SidebarItem::Library; // 搜索时切换到曲库视图
            let list = this
                .song_view
                .update(cx, |view, cx| view.search(&evt.query, cx));
            cx.global_mut::<Player>().set_playlist(list);
            cx.notify();
        })
        .detach();

        //订阅playbar事件
        cx.subscribe(&play_bar, |this, _that, _evt: &PlayBarMessage, cx| {
            this.now_playing.update(cx, |now_playing, cx| {
                now_playing.show(cx);
            });
            cx.notify();
        })
        .detach();

        // 订阅清除搜索事件
        cx.subscribe(&title_bar, |this, _that, _evt: &ClearSearchEvent, cx| {
            this.view_type = SidebarItem::Library;
            let list = this.song_view.update(cx, |view, cx| {
                view.clear_search(cx);
                view.set_view_type(ViewType::Library, cx)
            });
            cx.global_mut::<Player>().set_playlist(list);
            cx.notify();
        })
        .detach();

        // 订阅侧边栏消息 - 通过 song_view 来访问 library_state
        cx.subscribe(&sidebar, |this, _that, evt, cx| match evt {
            SidebarItem::Settings => {
                this.view_type = SidebarItem::Settings;
                cx.notify();
            }
            SidebarItem::Library => {
                this.view_type = SidebarItem::Library;
                let list = this
                    .song_view
                    .update(cx, |view, cx| view.set_view_type(ViewType::Library, cx));
                cx.global_mut::<Player>().set_playlist(list);
                cx.notify();
            }
            SidebarItem::Favorite => {
                this.view_type = SidebarItem::Favorite;
                let list = this
                    .song_view
                    .update(cx, |view, cx| view.set_view_type(ViewType::Favorite, cx));
                cx.global_mut::<Player>().set_playlist(list);
                cx.notify();
            }
            SidebarItem::History => {
                this.view_type = SidebarItem::History;
                let list = this
                    .song_view
                    .update(cx, |view, cx| view.set_view_type(ViewType::History, cx));
                cx.global_mut::<Player>().set_playlist(list);
                cx.notify();
            }
            SidebarItem::Custom(_) => {
                // TODO: 处理自定义歌单
            }
        })
        .detach();

        Self {
            view_type: SidebarItem::Library,
            song_view,
            setting,
            play_bar,
            title_bar,
            sidebar,
            now_playing: player_detail,
        }
    }
}

impl Render for Zotu {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_row()
            .relative()
            .bg(rgb(0xFAFAFA))
            .when_else(
                self.now_playing.read(cx).is_showing(),
                // 显示播放器详情页
                |this| this.child(self.now_playing.clone()),
                |this| {
                    this.child(self.sidebar.clone()).child(
                        div()
                            .h_full()
                            .w_full()
                            .flex()
                            .flex_col()
                            .child(self.title_bar.clone())
                            .map(|parent| match self.view_type {
                                SidebarItem::Settings => parent.child(self.setting.clone()),
                                SidebarItem::Library
                                | SidebarItem::Favorite
                                | SidebarItem::History
                                | SidebarItem::Custom(_) => parent
                                    .child(self.song_view.clone())
                                    .child(self.play_bar.clone()),
                            }),
                    )
                },
            )
    }
}


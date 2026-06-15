use gpui::{prelude::FluentBuilder, *};

use crate::{
    application::{AppController, AppEvent, AppRoute},
    theme::*,
};

use super::{AlbumList, NowPlaying, PlayBar, Settings, Sidebar, TitleBar};

pub struct Zotu {
    controller: Entity<AppController>,
    album_list: Entity<AlbumList>,
    settings: Entity<Settings>,
    play_bar: Entity<PlayBar>,
    title_bar: Entity<TitleBar>,
    sidebar: Entity<Sidebar>,
    now_playing: Entity<NowPlaying>,
}

impl Zotu {
    pub fn new(
        _window: &mut Window,
        controller: Entity<AppController>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.subscribe(&controller, |_this, _controller, event, cx| {
            if matches!(
                event,
                AppEvent::NavigationChanged | AppEvent::PlayerDetailsChanged
            ) {
                cx.notify();
            }
        })
        .detach();

        Self {
            album_list: cx.new(|cx| AlbumList::new(controller.clone(), cx)),
            settings: cx.new(|cx| Settings::new(controller.clone(), cx)),
            play_bar: cx.new(|cx| PlayBar::new(controller.clone(), cx)),
            title_bar: cx.new(|cx| TitleBar::new(controller.clone(), cx)),
            sidebar: cx.new(|cx| Sidebar::new(controller.clone(), cx)),
            now_playing: cx.new(|cx| NowPlaying::new(controller.clone(), cx)),
            controller,
        }
    }
}

impl Render for Zotu {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.controller.read(cx).state();
        let route = state.route();
        let player_details_open = state.player_details_open();

        div()
            .size_full()
            .flex()
            .flex_row()
            .relative()
            .bg(bg_app())
            .text_color(text_primary())
            .when_else(
                player_details_open,
                |this| this.child(self.now_playing.clone()),
                |this| {
                    this.child(self.sidebar.clone()).child(
                        div()
                            .h_full()
                            .w_full()
                            .flex()
                            .flex_col()
                            .child(self.title_bar.clone())
                            .map(|parent| match route {
                                AppRoute::Settings => parent.child(self.settings.clone()),
                                AppRoute::Library | AppRoute::Favorite | AppRoute::History => {
                                    parent
                                        .child(self.album_list.clone())
                                        .child(self.play_bar.clone())
                                }
                            }),
                    )
                },
            )
    }
}

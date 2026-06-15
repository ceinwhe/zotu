use gpui::{prelude::FluentBuilder, *};

use crate::{
    application::{
        AlbumInfo, AppAction, AppController, AppEvent, LoopMode, PlayState, PlaybackAction,
    },
    theme::*,
};

use super::format_duration;

pub struct NowPlaying {
    controller: Entity<AppController>,
}

impl NowPlaying {
    pub fn new(controller: Entity<AppController>, cx: &mut Context<Self>) -> Self {
        cx.subscribe(&controller, |_this, _controller, event, cx| {
            if *event == AppEvent::PlaybackChanged {
                cx.notify();
            }
        })
        .detach();
        Self { controller }
    }

    fn dispatch(&self, action: AppAction, cx: &mut Context<Self>) {
        self.controller.update(cx, |controller, cx| {
            controller.dispatch(action, cx);
        });
    }

    fn render_cover(&self, track: Option<&AlbumInfo>) -> impl IntoElement {
        div().flex().items_center().justify_center().child(
            div()
                .size(Pixels::from(COVER_LARGE_SIZE))
                .rounded_xl()
                .shadow_lg()
                .bg(bg_hover())
                .flex()
                .items_center()
                .justify_center()
                .overflow_hidden()
                .when_some(track.and_then(|value| value.cover_path()), |this, path| {
                    this.child(img(path.replace("D:/Code/zotu/assets", "")).size_full())
                })
                .when(
                    track.and_then(|value| value.cover_path()).is_none(),
                    |this| {
                        this.child(
                            svg()
                                .path("svg/album.svg")
                                .size(Pixels::from(64.0))
                                .text_color(text_muted()),
                        )
                    },
                ),
        )
    }
}

impl Render for NowPlaying {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let playback = self.controller.read(cx).state().playback().clone();
        let track = playback.current_track.as_ref();
        let (title, artist, album) = track
            .map(|track| {
                (
                    track.title().to_string(),
                    track.artist().to_string(),
                    track.album().to_string(),
                )
            })
            .unwrap_or_else(|| ("未播放".to_string(), String::new(), String::new()));
        let (elapsed, duration, progress) = playback
            .progress
            .as_ref()
            .map(|value| {
                (
                    format_duration(value.elapsed),
                    format_duration(value.duration),
                    value.progress,
                )
            })
            .unwrap_or_else(|| ("0:00".to_string(), "0:00".to_string(), 0.0));
        let play_icon = match playback.play_state {
            PlayState::Play => "svg/pause.svg",
            PlayState::Paused | PlayState::Stopped => "svg/play.svg",
        };
        let loop_icon = match playback.loop_mode {
            LoopMode::List => "svg/list.svg",
            LoopMode::Single => "svg/single.svg",
            LoopMode::Random => "svg/random.svg",
        };

        div()
            .id("player-detail")
            .absolute()
            .size_full()
            .bg(bg_content())
            .flex()
            .flex_col()
            .child(
                div().flex().items_start().justify_start().p_4().child(
                    div()
                        .id("close-btn")
                        .size(px(40.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .bg(bg_hover())
                        .cursor_pointer()
                        .hover(|style| style.bg(bg_active()))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _evt, _window, cx| {
                                this.dispatch(AppAction::SetPlayerDetailsOpen(false), cx);
                            }),
                        )
                        .child(
                            svg()
                                .path("svg/close.svg")
                                .size(px(24.0))
                                .text_color(text_primary()),
                        ),
                ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .flex_1()
                    .child(self.render_cover(track))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_2()
                            .mt_6()
                            .child(
                                div()
                                    .text_xl()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(text_primary())
                                    .child(title),
                            )
                            .child(div().text_base().text_color(text_tertiary()).child(artist))
                            .child(div().text_sm().text_color(text_placeholder()).child(album)),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .px_12()
                            .child(
                                div()
                                    .w_full()
                                    .h(Pixels::from(4.0))
                                    .bg(bg_hover())
                                    .rounded_full()
                                    .child(
                                        div()
                                            .h_full()
                                            .w(relative(progress))
                                            .bg(accent_blue())
                                            .rounded_full(),
                                    ),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .flex_row()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(text_placeholder())
                                            .child(elapsed),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(text_placeholder())
                                            .child(duration),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_center()
                            .gap_8()
                            .mt_6()
                            .child(detail_button(loop_icon, 6.0).on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _evt, _window, cx| {
                                    this.dispatch(
                                        AppAction::Playback(PlaybackAction::CycleLoopMode),
                                        cx,
                                    );
                                }),
                            ))
                            .child(detail_button("svg/last.svg", 6.0).on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _evt, _window, cx| {
                                    this.dispatch(
                                        AppAction::Playback(PlaybackAction::Previous),
                                        cx,
                                    );
                                }),
                            ))
                            .child(detail_button(play_icon, 10.0).on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _evt, _window, cx| {
                                    this.dispatch(AppAction::Playback(PlaybackAction::Toggle), cx);
                                }),
                            ))
                            .child(detail_button("svg/next.svg", 10.0).on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _evt, _window, cx| {
                                    this.dispatch(AppAction::Playback(PlaybackAction::Next), cx);
                                }),
                            )),
                    ),
            )
    }
}

fn detail_button(path: &'static str, size: f32) -> Svg {
    svg()
        .path(path)
        .size(Pixels::from(size * 4.0))
        .text_color(text_primary())
        .rounded_full()
        .cursor_pointer()
}

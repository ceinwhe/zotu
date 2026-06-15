use std::sync::Arc;

use gpui::{prelude::FluentBuilder, *};

use crate::{
    application::{AppAction, AppController, AppEvent, LoopMode, PlayState, PlaybackAction},
    theme::*,
};

use super::format_duration;

pub struct PlayBar {
    controller: Entity<AppController>,
}

impl PlayBar {
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
}

impl Render for PlayBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let playback = self.controller.read(cx).state().playback().clone();
        let (title, artist) = playback
            .current_track
            .as_ref()
            .map(|track| (track.title().to_string(), track.artist().to_string()))
            .unwrap_or_else(|| ("未播放".to_string(), String::new()));
        let cover = playback
            .current_track
            .as_ref()
            .and_then(|track| track.cover_64());
        let play_icon = match playback.play_state {
            PlayState::Play => "svg/pause.svg",
            PlayState::Paused | PlayState::Stopped => "svg/play.svg",
        };
        let loop_icon = match playback.loop_mode {
            LoopMode::List => "svg/list.svg",
            LoopMode::Single => "svg/single.svg",
            LoopMode::Random => "svg/random.svg",
        };
        let progress = playback.progress;

        div()
            .w_full()
            .h(Pixels::from(PLAYBAR_HEIGHT))
            .bg(bg_playlist())
            .flex()
            .flex_col()
            .flex_shrink_0()
            .border_t_1()
            .border_color(border_light())
            .child(
                div()
                    .w_full()
                    .h(Pixels::from(3.0))
                    .bg(bg_hover())
                    .flex_shrink_0()
                    .child(
                        div()
                            .h_full()
                            .w(relative(
                                progress.as_ref().map(|value| value.progress).unwrap_or(0.0),
                            ))
                            .bg(accent_blue()),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .flex_1()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .id("playbar-song-info")
                            .flex()
                            .flex_row()
                            .gap_1()
                            .cursor_pointer()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _evt, _window, cx| {
                                    this.dispatch(AppAction::SetPlayerDetailsOpen(true), cx);
                                }),
                            )
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .size(Pixels::from(COVER_THUMB_SIZE))
                                    .ml_3()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded_md()
                                    .when_some(cover.clone(), |this, cover| {
                                        this.child(
                                            img(Arc::new(Image::from_bytes(
                                                ImageFormat::Jpeg,
                                                cover.to_vec(),
                                            )))
                                            .size_full()
                                            .rounded_md(),
                                        )
                                    })
                                    .when_none(&cover, |this| {
                                        this.child(
                                            svg()
                                                .path("svg/album.svg")
                                                .size_full()
                                                .text_color(text_muted()),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .justify_center()
                                    .w(Pixels::from(200.0))
                                    .overflow_hidden()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::MEDIUM)
                                            .truncate()
                                            .text_color(text_primary())
                                            .child(title),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(text_tertiary())
                                            .truncate()
                                            .child(artist),
                                    )
                                    .child(
                                        div().text_xs().text_color(text_placeholder()).child(
                                            progress
                                                .as_ref()
                                                .map(|value| {
                                                    format!(
                                                        "{} / {}",
                                                        format_duration(value.elapsed),
                                                        format_duration(value.duration)
                                                    )
                                                })
                                                .unwrap_or_else(|| "--:-- / --:--".to_string()),
                                        ),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_center()
                            .items_center()
                            .gap_5()
                            .mr_5()
                            .child(control_button(loop_icon, 6.0).on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _evt, _window, cx| {
                                    this.dispatch(
                                        AppAction::Playback(PlaybackAction::CycleLoopMode),
                                        cx,
                                    );
                                }),
                            ))
                            .child(control_button("svg/last.svg", 8.0).on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _evt, _window, cx| {
                                    this.dispatch(
                                        AppAction::Playback(PlaybackAction::Previous),
                                        cx,
                                    );
                                }),
                            ))
                            .child(control_button(play_icon, 8.0).on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _evt, _window, cx| {
                                    this.dispatch(AppAction::Playback(PlaybackAction::Toggle), cx);
                                }),
                            ))
                            .child(control_button("svg/next.svg", 8.0).on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _evt, _window, cx| {
                                    this.dispatch(AppAction::Playback(PlaybackAction::Next), cx);
                                }),
                            )),
                    ),
            )
    }
}

fn control_button(path: &'static str, size: f32) -> Svg {
    svg()
        .path(path)
        .size(Pixels::from(size * 4.0))
        .text_color(text_primary())
        .cursor_pointer()
}

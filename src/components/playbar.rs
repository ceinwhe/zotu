use crate::{
    play::player::{LoopMode, PlayState, Player},
    theme::*,
    util::format_duration,
};
use gpui::{prelude::FluentBuilder, *};
use std::sync::Arc;

pub enum PlayBarMessage {
    NowPlayingClick,
}

pub struct PlayBar {
    /// 用于定时刷新 UI 的异步任务
    _poll_task: Option<Task<()>>,
}

impl EventEmitter<PlayBarMessage> for PlayBar {}

impl PlayBar {
    pub fn new(cx: &mut Context<Self>) -> Self {
        // 启动智能轮询：仅在播放时以 250ms 间隔刷新，暂停时停止
        let task = cx.spawn(async move |this: WeakEntity<PlayBar>, cx: &mut AsyncApp| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(250))
                    .await;

                // 更新 Player 状态（检查自动下一首）
                let should_notify = cx
                    .update(|cx: &mut App| {
                        cx.update_global::<Player, _>(|player: &mut Player, _cx: &mut App| {
                            player.check_and_auto_next();
                            // 只在播放时通知 UI 刷新（节省资源）
                            player.is_playing()
                        })
                    })
                    .unwrap_or(false);

                if should_notify {
                    let result =
                        this.update(cx, |_this: &mut PlayBar, cx: &mut Context<PlayBar>| {
                            cx.notify();
                        });

                    if result.is_err() {
                        break;
                    }
                }
            }
        });

        PlayBar {
            _poll_task: Some(task),
        }
    }
}

impl Render for PlayBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let player = cx.global::<Player>();

        // 获取当前播放信息
        let current_track = player.current_track();
        let (title, artist) = match &current_track {
            Some(track) => (track.title().to_string(), track.artist().to_string()),
            None => ("未播放".to_string(), String::new()),
        };

        let play_btn_path = match player.play_state() {
            PlayState::Play => "svg/pause.svg",
            PlayState::Paused | PlayState::Stopped => "svg/play.svg",
        };

        let loop_mode_icon = match player.loop_mode() {
            LoopMode::List => "svg/list.svg",
            LoopMode::Single => "svg/single.svg",
            LoopMode::Random => "svg/random.svg",
        };

        let cover_64 = current_track.as_ref().and_then(|t| t.cover_64());

        // 获取播放进度
        let progress = player.progress();

        div()
            .w_full()
            .h(Pixels::from(PLAYBAR_HEIGHT))
            .bg(bg_playlist())
            .flex()
            .flex_col()
            .flex_shrink_0()
            .border_t_1()
            .border_color(border_light())
            // 进度条
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
                                progress.as_ref().map(|p| p.progress).unwrap_or(0.0),
                            ))
                            .bg(accent_blue()),
                    ),
            )
            // 主内容区
            .child(
                div()
                    .w_full()
                    .flex_1()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    // 歌曲信息区域
                    .child(
                        div()
                            .id("playbar-song-info")
                            .flex()
                            .flex_row()
                            .gap_1()
                            // 专辑封面
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .size(Pixels::from(COVER_THUMB_SIZE))
                                    .ml_3()
                                    .flex()
                                    .content_center()
                                    .justify_center()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .when_else(
                                        cover_64.is_some(),
                                        |this| {
                                            this.child(
                                                img(Arc::new(Image::from_bytes(
                                                    ImageFormat::Jpeg,
                                                    cover_64.unwrap().to_vec(),
                                                )))
                                                .size_full()
                                                .rounded_md(),
                                            )
                                        },
                                        |this| {
                                            this.child(
                                                svg()
                                                    .path("svg/album.svg")
                                                    .size_full()
                                                    .text_color(text_muted()),
                                            )
                                        },
                                    )
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|_this, _evt, _window, cx| {
                                            cx.emit::<PlayBarMessage>(
                                                PlayBarMessage::NowPlayingClick,
                                            );
                                        }),
                                    ),
                            )
                            // 标题和艺术家信息 + 时间
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
                                    .child(div().text_xs().text_color(text_placeholder()).child(
                                        match &progress {
                                            Some(p) => format!(
                                                "{} / {}",
                                                format_duration(p.elapsed),
                                                format_duration(p.duration)
                                            ),
                                            None => "--:-- / --:--".to_string(),
                                        },
                                    )),
                            ),
                    )
                    // 播放控制按钮区域
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_center()
                            .items_center()
                            .gap_5()
                            .mr_5()
                            // 循环模式
                            .child(
                                svg()
                                    .path(loop_mode_icon)
                                    .size_6()
                                    .text_color(text_secondary())
                                    .cursor_pointer()
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|_this, _evt, _window, cx| {
                                            cx.update_global::<Player, _>(|player, _cx| {
                                                player.toggle_loop_mode();
                                            });
                                        }),
                                    ),
                            )
                            // 上一首
                            .child(
                                svg()
                                    .path("svg/last.svg")
                                    .text_color(text_primary())
                                    .cursor_pointer()
                                    .size_8()
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|_this, _evt, _window, cx| {
                                            cx.update_global::<Player, _>(|player, _cx| {
                                                player.previous();
                                            });
                                        }),
                                    ),
                            )
                            // 播放/暂停
                            .child(
                                svg()
                                    .path(play_btn_path)
                                    .size_8()
                                    .text_color(text_primary())
                                    .cursor_pointer()
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|_this, _evt, _window, cx| {
                                            cx.update_global::<Player, _>(|player, _cx| {
                                                player.toggle_play();
                                            });
                                        }),
                                    ),
                            )
                            // 下一首
                            .child(
                                svg()
                                    .path("svg/next.svg")
                                    .text_color(text_primary())
                                    .cursor_pointer()
                                    .size_8()
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|_this, _evt, _window, cx| {
                                            cx.update_global::<Player, _>(|player, _cx| {
                                                player.next();
                                            })
                                        }),
                                    ),
                            ),
                    ),
            )
    }
}

use crate::play::player::{LoopMode, PlayState, Player};
use gpui::*;

pub struct PlayBar {
    // 用于定时刷新 UI
    _poll_task: Option<Task<()>>,
}

impl PlayBar {
    pub fn new(cx: &mut Context<Self>) -> Self {
        // 启动定时器轮询播放状态
        let task = cx.spawn(async move |this: WeakEntity<PlayBar>, cx: &mut AsyncApp| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(500))
                    .await;

                // 检查是否需要自动播放下一首
                let result = cx.update(|cx: &mut App| {
                    cx.update_global::<Player, _>(|player: &mut Player, _cx: &mut App| {
                        player.check_and_auto_next();
                    });
                });

                if result.is_err() {
                    break;
                }

                // 通知 UI 刷新
                let result = this.update(cx, |_this: &mut PlayBar, cx: &mut Context<PlayBar>| {
                    cx.notify();
                });

                if result.is_err() {
                    break;
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
        let (title, artist) = match player.current_track() {
            Some(track) => (track.title.to_string(), track.artist.to_string()),
            None => ("未播放".to_string(), String::new()),
        };

        let play_btn_path = match player.play_state() {
            PlayState::Play => "pause.svg",  // 正在播放时显示暂停按钮
            PlayState::Paused => "play.svg", // 已暂停时显示播放按钮
        };

        let loop_mode = match player.loop_mode() {
            LoopMode::List => "list.svg",
            LoopMode::Single => "single.svg",
            LoopMode::Random => "random.svg",
        };

        div()
            .w_full()
            .h(Pixels::from(80.0))
            .bg(rgb(0xF5F5F5))
            .flex()
            .flex_row()
            //防止被压缩
            .flex_shrink_0()
            .items_center()
            .justify_between()
            .border_t_1()
            .border_color(rgb(0xDDDDDD))
            // 歌曲信息区域
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
                            .child(title),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x666666))
                            .truncate()
                            .child(artist),
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
                    // 上一首按钮
                    .child(
                        svg()
                            .path("last.svg")
                            .text_color(black())
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
                    // 播放/暂停按钮
                    .child(
                        svg()
                            .path(play_btn_path)
                            .size_8()
                            .text_color(black())
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
                    // 下一首按钮
                    .child(
                        svg()
                            .path("next.svg")
                            .text_color(black())
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
                    )
                    // 循环模式按钮
                    .child(
                        svg()
                            .path(loop_mode)
                            .size_6()
                            .text_color(black())
                            .cursor_pointer()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|_this, _evt, _window, cx| {
                                    cx.update_global::<Player, _>(|player, _cx| {
                                        player.toggle_loop_mode();
                                    });
                                }),
                            ),
                    ),
            )
    }
}

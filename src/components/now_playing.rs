use crate::{
    db::metadata::AlbumInfo,
    play::player::{LoopMode, PlayState, Player},
};
use gpui::{prelude::FluentBuilder, *};

/// 播放器详情页状态
pub struct PlayerDetail {
    /// 是否显示详情页
    show: bool,
}

impl PlayerDetail {
    pub fn new() -> Self {
        PlayerDetail { show: false }
    }

    /// 是否正在显示
    pub fn is_showing(&self) -> bool {
        self.show
    }

    /// 显示详情页
    pub fn show(&mut self, cx: &mut Context<Self>) {
        self.show = true;
        cx.notify();
        // 加载当前歌曲的歌词
    }

    /// 隐藏详情页
    fn hide(&mut self, cx: &mut Context<Self>) {
        self.show = false;
        cx.notify();
    }

    /// 渲染封面区域
    fn render_cover(&self, track: Option<&AlbumInfo>) -> impl IntoElement {
        div().flex().items_center().justify_center().child(
            // 大封面
            div()
                .rounded_xl()
                .shadow_lg()
                .bg(rgb(0xE5E7EB))
                .flex()
                .items_center()
                .justify_center()
                .when_some(
                    track.and_then(|this| this.cover_path()),
                    |this, cover_path| {
                        this.child(img(cover_path.replace("D:/Code/zotu/assets", "")).size_full())
                    },
                )
                .when(
                    track.and_then(|this| this.cover_path()).is_none(),
                    |this| this.child(svg().path("svg/album.svg").size_full().text_color(black())),
                ),
        )
    }

    /// 渲染播放控制区域
    fn render_controls(&self, cx: &Context<Self>) -> impl IntoElement {
        let player = cx.global::<Player>();

        let play_btn_path = match player.play_state() {
            PlayState::Play => "svg/pause.svg",
            PlayState::Paused => "svg/play.svg",
        };

        let loop_mode = match player.loop_mode() {
            LoopMode::List => "svg/list.svg",
            LoopMode::Single => "svg/single.svg",
            LoopMode::Random => "svg/random.svg",
        };

        div().child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_center()
                .gap_8()
                // 循环模式
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
                )
                // 上一首
                .child(
                    svg()
                        .path("svg/last.svg")
                        .size_6()
                        .text_color(rgb(0x374151))
                        .cursor_pointer()
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
                        .size_6()
                        .text_color(black())
                        .rounded_full()
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
                        .size_10()
                        .text_color(rgb(0x374151))
                        .cursor_pointer()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|_this, _evt, _window, cx| {
                                cx.update_global::<Player, _>(|player, _cx| {
                                    player.next();
                                });
                            }),
                        ),
                ),
        )
    }
}

impl Render for PlayerDetail {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 获取当前播放的歌曲信息
        let player = cx.global::<Player>();
        let current_track = player.current_track().cloned();

        div()
            .id("player-detail")
            .bg(rgb(0xFCFCFC))
            .flex()
            .flex_col()
            .child(
                div().flex().items_start().justify_start().child(
                    svg()
                        .path("svg/close.svg")
                        .id("close")
                        .size_6()
                        .text_color(rgb(0x6B7280))
                        .rounded_full()
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(0xF3F4F6)))
                        .on_click(cx.listener(move |this, _evt, _window, cx| {
                            this.hide(cx);
                        })),
                ),
            )
            // 主内容区域
            .child(
                div()
                    .flex()
                    .size(Pixels::from(200.0))
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    // 左侧封面区域
                    .child(self.render_cover(current_track.as_ref())), // 右侧歌词区域
            )
            // 底部播放控制
            .child(
                div()
                    .flex()
                    .justify_center()
                    .items_center()
                    .child(self.render_controls(cx)),
            )
    }
}

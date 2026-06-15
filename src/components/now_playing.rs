use crate::{
    db::metadata::AlbumInfo,
    play::player::{LoopMode, PlayState, Player},
    theme::*,
    util::format_duration,
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
    }

    /// 隐藏详情页
    fn hide(&mut self, cx: &mut Context<Self>) {
        self.show = false;
        cx.notify();
    }

    /// 渲染封面区域（优先使用原始封面文件，回退到缩略图）
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
                .when_some(track.and_then(|t| t.cover_path()), |this, cover_path| {
                    // 使用原始封面文件显示大图
                    this.child(img(cover_path.replace("D:/Code/zotu/assets", "")).size_full())
                })
                .when(track.and_then(|t| t.cover_path()).is_none(), |this| {
                    this.child(
                        svg()
                            .path("svg/album.svg")
                            .size(Pixels::from(64.0))
                            .text_color(text_muted()),
                    )
                }),
        )
    }

    /// 渲染歌曲信息
    fn render_track_info(&self, track: Option<&AlbumInfo>) -> impl IntoElement {
        let (title, artist, album) = match track {
            Some(t) => (
                t.title().to_string(),
                t.artist().to_string(),
                t.album().to_string(),
            ),
            None => ("未播放".to_string(), String::new(), String::new()),
        };

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
            .child(div().text_sm().text_color(text_placeholder()).child(album))
    }

    /// 渲染播放进度
    fn render_progress(&self, cx: &Context<Self>) -> impl IntoElement {
        let player = cx.global::<Player>();
        let progress = player.progress();

        let (elapsed_str, duration_str, progress_pct) = match &progress {
            Some(p) => (
                format_duration(p.elapsed),
                format_duration(p.duration),
                p.progress,
            ),
            None => ("0:00".to_string(), "0:00".to_string(), 0.0f32),
        };

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap_2()
            .px_12()
            .child(
                // 进度条
                div()
                    .w_full()
                    .h(Pixels::from(4.0))
                    .bg(bg_hover())
                    .rounded_full()
                    .cursor_pointer()
                    .child(
                        div()
                            .h_full()
                            .w(relative(progress_pct))
                            .bg(accent_blue())
                            .rounded_full(),
                    ),
            )
            .child(
                // 时间标签
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .text_color(text_placeholder())
                            .child(elapsed_str),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(text_placeholder())
                            .child(duration_str),
                    ),
            )
    }

    /// 渲染播放控制区域
    fn render_controls(&self, cx: &Context<Self>) -> impl IntoElement {
        let player = cx.global::<Player>();

        let play_btn_path = match player.play_state() {
            PlayState::Play => "svg/pause.svg",
            PlayState::Paused | PlayState::Stopped => "svg/play.svg",
        };

        let loop_mode_icon = match player.loop_mode() {
            LoopMode::List => "svg/list.svg",
            LoopMode::Single => "svg/single.svg",
            LoopMode::Random => "svg/random.svg",
        };

        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_center()
            .gap_8()
            .mt_6()
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
                    .size_6()
                    .text_color(text_secondary())
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
                    .size_10()
                    .text_color(text_primary())
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
                    .text_color(text_secondary())
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|_this, _evt, _window, cx| {
                            cx.update_global::<Player, _>(|player, _cx| {
                                player.next();
                            });
                        }),
                    ),
            )
    }
}

impl Render for PlayerDetail {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let player = cx.global::<Player>();
        let current_track = player.current_track().cloned();

        div()
            .id("player-detail")
            .absolute()
            .size_full()
            .bg(bg_content())
            .flex()
            .flex_col()
            // 关闭按钮
            .child(
                div().flex().items_start().justify_start().p_4().child(
                    svg()
                        .path("svg/close.svg")
                        .id("close")
                        .size_6()
                        .text_color(text_tertiary())
                        .rounded_full()
                        .cursor_pointer()
                        .hover(|s| s.bg(bg_active()))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _evt, _window, cx| {
                                this.hide(cx);
                            }),
                        ),
                ),
            )
            // 主内容区域
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .flex_1()
                    // 封面
                    .child(self.render_cover(current_track.as_ref()))
                    // 歌曲信息
                    .child(self.render_track_info(current_track.as_ref()))
                    // 进度条
                    .child(self.render_progress(cx))
                    // 播放控制
                    .child(self.render_controls(cx)),
            )
    }
}

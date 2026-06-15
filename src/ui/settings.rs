use gpui::*;
use rfd::AsyncFileDialog;

use crate::{
    application::{AppAction, AppController, AppEvent, LibraryAction},
    theme::*,
};

pub struct Settings {
    controller: Entity<AppController>,
}

impl Settings {
    pub fn new(controller: Entity<AppController>, cx: &mut Context<Self>) -> Self {
        cx.subscribe(&controller, |_this, _controller, event, cx| {
            if *event == AppEvent::SettingsChanged {
                cx.notify();
            }
        })
        .detach();
        Self { controller }
    }

    fn pick_music_folder(&self, cx: &mut Context<Self>) {
        let controller = self.controller.clone();
        cx.spawn(
            async move |_this: WeakEntity<Settings>, cx: &mut AsyncApp| {
                let folder = AsyncFileDialog::new()
                    .set_title("选择音乐文件夹")
                    .pick_folder()
                    .await;

                if let Some(folder) = folder {
                    let path = folder.path().to_string_lossy().to_string();
                    let _ = controller.update(cx, |controller, cx| {
                        controller
                            .dispatch(AppAction::Library(LibraryAction::ScanDirectory(path)), cx);
                    });
                }
            },
        )
        .detach();
    }
}

impl Render for Settings {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let music_directory = self
            .controller
            .read(cx)
            .state()
            .music_directory()
            .to_string();

        div()
            .flex()
            .flex_col()
            .p_6()
            .bg(bg_content())
            .child(
                div()
                    .mb_6()
                    .text_2xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(text_primary())
                    .child("设置"),
            )
            .child(
                div()
                    .mb_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(text_secondary())
                            .mb_2()
                            .child("音乐文件夹"),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .flex_1()
                                    .h(px(SETTING_ITEM_HEIGHT))
                                    .bg(bg_input())
                                    .rounded_lg()
                                    .flex()
                                    .items_center()
                                    .px_4()
                                    .text_sm()
                                    .text_color(text_secondary())
                                    .truncate()
                                    .child(music_directory),
                            )
                            .child(
                                div()
                                    .h(px(SETTING_ITEM_HEIGHT))
                                    .px_4()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .bg(accent_blue())
                                    .rounded_lg()
                                    .cursor_pointer()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(text_primary())
                                    .hover(|style| style.bg(bg_active()))
                                    .child("更改")
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _evt, _window, cx| {
                                            this.pick_music_folder(cx);
                                        }),
                                    ),
                            ),
                    ),
            )
            .child(
                div()
                    .mt_auto()
                    .pt_4()
                    .border_t_1()
                    .border_color(border_default())
                    .text_xs()
                    .text_color(text_placeholder())
                    .child("Zotu Music Player v0.1.0"),
            )
    }
}

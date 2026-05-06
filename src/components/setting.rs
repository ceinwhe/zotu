use gpui::*;
use rfd::AsyncFileDialog;

use crate::{config::Config, db::database::DB, theme::*};

pub struct Setting;

impl Setting {
    /// 打开文件夹选择对话框并更新音乐目录
    fn pick_music_folder(&self, cx: &mut Context<Self>) {
        cx.spawn(async move |_this: WeakEntity<Setting>, cx: &mut AsyncApp| {
            let folder = AsyncFileDialog::new()
                .set_title("选择音乐文件夹")
                .pick_folder()
                .await;

            if let Some(folder) = folder {
                let path = folder.path().to_string_lossy().to_string();
                cx.update(|cx: &mut App| {
                    // 更新配置
                    cx.update_global::<Config, _>(|config, _cx| {
                        config.media_file.music_directory = SharedString::new(path.clone());
                    });

                    // 扫描并添加到数据库
                    cx.update_global::<DB, _>(|db, _cx| {
                        if let Err(e) = db.add_metadata_to_library(&path) {
                            eprintln!("[WARN] 扫描音乐文件夹失败: {}", e);
                        }
                    });
                })
                .ok();
            }
        })
        .detach();
    }
}

impl Render for Setting {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let music_dir = cx.global::<Config>().media_file.music_directory.to_string();

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
            // 媒体文件目录设置
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
                                    .child(music_dir.clone()),
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
                                    .hover(|s| s.bg(bg_active()))
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
            // 关于信息
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

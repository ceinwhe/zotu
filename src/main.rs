use gpui::*;
use zotu::{app::Zotu, assets::Assets, play::player::Player, db::db::DB};

fn main() {
    let window_options = WindowOptions {
        titlebar: Some(TitlebarOptions {
            title: Some(SharedString::new("Zotu")),
            appears_transparent: true,
            ..Default::default()
        }),
        is_movable: true,
        is_resizable: true,
        ..Default::default()
    };

    Application::new()
        .with_assets(Assets::new("./assets"))
        .run(move |cx: &mut App| {
            // 初始化全局播放器
            cx.set_global(Player::new());
            // 初始化全局数据库连接 unwrap需要改进错误处理
            cx.set_global(DB::new("metadata.db").unwrap());

            cx.open_window(window_options, |window, cx| {
                cx.new(|cx| Zotu::new(window, cx))
            })
            .unwrap();
        });
}

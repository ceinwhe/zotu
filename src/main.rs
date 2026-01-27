use gpui::*;
use zotu::{app::Zotu, assets::Assets, config::Config, db::db::DB, play::player::Player};

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
            // 读取或创建配置文件
            cx.set_global(Config::load_or_create("config.json").unwrap());
            cx.set_global(Player::new());
            // 初始化全局数据库连接 unwrap需要改进错误处理
            cx.set_global(DB::new("metadata.db").unwrap());

            cx.open_window(window_options, |window, cx| {
                cx.new(|cx| Zotu::new(window, cx))
            })
            .unwrap();

            // 在应用关闭时保存配置
            cx.on_window_closed(move |app| {
                app.global::<Config>().save("config.json").unwrap();
            })
            .detach();
        });
}

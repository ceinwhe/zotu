use gpui::*;
use zotu::{
    app::Zotu, assets::Assets, audio::ffmpeg::FfmpegEngine, audio::player::Player,
    config::Config, db::database::DB, error::log_error,
};

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
            // 读取或创建配置文件（失败时使用默认配置）
            let config = match Config::load_or_create("config.json") {
                Ok(cfg) => cfg,
                Err(e) => {
                    log_error(&e, "加载配置文件失败，使用默认配置");
                    Config::default()
                }
            };
            cx.set_global(config);

            cx.set_global(Player::new(Box::new(FfmpegEngine::new().expect("Failed to create audio engine"))));

            // 初始化全局数据库连接（失败时退出）
            match DB::new("metadata.db") {
                Ok(db) => cx.set_global(db),
                Err(e) => {
                    log_error(&e, "数据库初始化失败，应用无法启动");
                    eprintln!("[FATAL] 数据库连接失败: {}", e);
                    return;
                }
            };

            match cx.open_window(window_options, |window, cx| {
                cx.new(|cx| Zotu::new(window, cx))
            }) {
                Ok(_) => {}
                Err(e) => {
                    log_error(&e, "打开窗口失败");
                    return;
                }
            }

            // 在应用关闭时保存配置（忽略保存错误）
            cx.on_window_closed(move |app| {
                if let Err(e) = app.global::<Config>().save("config.json") {
                    eprintln!("[WARN] 保存配置失败: {}", e);
                }
            })
            .detach();
        });
}

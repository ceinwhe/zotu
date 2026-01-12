use gpui::*;
use uuid::Uuid;
use zotu::{app::Zotu, assets::Assets, play::player::Player, util};

fn main() {
    let music_path = r"C:\Users\ceinw\OneDrive\Desktop\Music";
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

    let files = util::list_file(&music_path).unwrap_or_default();

    let audio_list = files
        .iter()
        .filter_map(|path| zotu::play::metadata::AlbumInfo::new(path, Uuid::new_v4()).ok())
        .collect::<Vec<_>>();

    Application::new()
        .with_assets(Assets::new("./assets"))
        .run(move |cx: &mut App| {

            // 初始化全局播放器
            cx.set_global(Player::new());

            cx.open_window(window_options, |window, cx| {
                cx.new(|cx| Zotu::new(window, cx, audio_list))
            })
            .unwrap();
        });
}

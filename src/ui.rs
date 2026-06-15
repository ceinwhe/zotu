mod album_list;
mod context_menu;
mod now_playing;
mod play_bar;
mod root;
mod search_box;
mod settings;
mod sidebar;
mod title_bar;

use album_list::AlbumList;
use context_menu::ContextMenu;
use now_playing::NowPlaying;
use play_bar::PlayBar;
pub use root::Zotu;
use search_box::SearchBox;
use settings::Settings;
use sidebar::Sidebar;
use title_bar::TitleBar;

fn format_duration(seconds: u64) -> String {
    let minutes = seconds / 60;
    let seconds = seconds % 60;
    format!("{minutes}:{seconds:02}")
}

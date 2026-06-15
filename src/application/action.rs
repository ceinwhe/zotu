use uuid::Uuid;

use super::state::AppRoute;

#[derive(Clone, Debug)]
pub enum AppAction {
    Navigate(AppRoute),
    SearchChanged(String),
    SearchCleared,
    SetPlayerDetailsOpen(bool),
    Playback(PlaybackAction),
    Library(LibraryAction),
}

#[derive(Clone, Debug)]
pub enum PlaybackAction {
    PlayTrack(Uuid),
    Toggle,
    Previous,
    Next,
    CycleLoopMode,
    Seek(u64),
}

#[derive(Clone, Debug)]
pub enum LibraryAction {
    AddFavorite(Uuid),
    RemoveFavorite(Uuid),
    ScanDirectory(String),
}

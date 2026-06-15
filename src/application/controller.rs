use std::time::Duration;

use gpui::*;

use crate::{audio::player::Player, config::Config};

use super::{
    action::{AppAction, LibraryAction, PlaybackAction},
    repository::LibraryRepository,
    state::{AppRoute, AppState, LibraryCatalog, PlaybackViewState},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppEvent {
    NavigationChanged,
    SearchChanged,
    LibraryChanged,
    PlaybackChanged,
    SettingsChanged,
    PlayerDetailsChanged,
}

pub struct AppController {
    state: AppState,
    player: Player,
    repository: Box<dyn LibraryRepository>,
    _poll_task: Task<()>,
}

impl EventEmitter<AppEvent> for AppController {}

impl AppController {
    pub fn new(
        repository: Box<dyn LibraryRepository>,
        mut player: Player,
        music_directory: String,
        cx: &mut Context<Self>,
    ) -> Self {
        let catalog = Self::load_catalog(repository.as_ref());
        player.set_playlist(catalog.library());

        let poll_task = cx.spawn(
            async move |this: WeakEntity<AppController>, cx: &mut AsyncApp| loop {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;

                let result = this.update(cx, |controller, cx| {
                    let was_playing = controller.player.is_playing();
                    controller.player.check_and_auto_next();
                    let changed = controller.sync_playback();
                    if was_playing || controller.player.is_playing() || changed {
                        cx.emit(AppEvent::PlaybackChanged);
                    }
                });
                if result.is_err() {
                    break;
                }
            },
        );

        let mut controller = Self {
            state: AppState::new(catalog, music_directory),
            player,
            repository,
            _poll_task: poll_task,
        };
        controller.sync_playback();
        controller
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn dispatch(&mut self, action: AppAction, cx: &mut Context<Self>) {
        match action {
            AppAction::Navigate(route) => self.navigate(route, cx),
            AppAction::SearchChanged(query) => self.search(query, cx),
            AppAction::SearchCleared => self.search(String::new(), cx),
            AppAction::SetPlayerDetailsOpen(open) => {
                self.state.set_player_details_open(open);
                cx.emit(AppEvent::PlayerDetailsChanged);
            }
            AppAction::Playback(action) => self.handle_playback(action, cx),
            AppAction::Library(action) => self.handle_library(action, cx),
        }
        cx.notify();
    }

    fn navigate(&mut self, route: AppRoute, cx: &mut Context<Self>) {
        self.state.set_route(route);
        self.sync_playlist();
        cx.emit(AppEvent::NavigationChanged);
        cx.emit(AppEvent::SearchChanged);
        cx.emit(AppEvent::LibraryChanged);
    }

    fn search(&mut self, query: String, cx: &mut Context<Self>) {
        self.state.set_search_query(query);
        self.sync_playlist();
        cx.emit(AppEvent::NavigationChanged);
        cx.emit(AppEvent::SearchChanged);
        cx.emit(AppEvent::LibraryChanged);
    }

    fn handle_playback(&mut self, action: PlaybackAction, cx: &mut Context<Self>) {
        match action {
            PlaybackAction::PlayTrack(id) => self.play_track(id, cx),
            PlaybackAction::Toggle => self.player.toggle_play(),
            PlaybackAction::Previous => self.player.previous(),
            PlaybackAction::Next => self.player.next(),
            PlaybackAction::CycleLoopMode => self.player.toggle_loop_mode(),
            PlaybackAction::Seek(position) => self.player.seek(position),
        }
        self.sync_playback();
        cx.emit(AppEvent::PlaybackChanged);
    }

    fn play_track(&mut self, id: uuid::Uuid, cx: &mut Context<Self>) {
        let Some(track) = self.state.track(&id).cloned() else {
            return;
        };

        if let Err(error) = self.repository.add_history(&id) {
            eprintln!("[WARN] Failed to write playback history: {error}");
        }
        self.state.catalog_mut().add_history(&id);
        self.state.rebuild_visible_tracks();
        self.player.play_track(&track);

        if self.state.route() == AppRoute::History {
            self.sync_playlist();
        }
        cx.emit(AppEvent::LibraryChanged);
    }

    fn handle_library(&mut self, action: LibraryAction, cx: &mut Context<Self>) {
        match action {
            LibraryAction::AddFavorite(id) => {
                if let Err(error) = self.repository.add_favorite(&id) {
                    eprintln!("[WARN] Failed to add favorite: {error}");
                    return;
                }
                self.state.catalog_mut().add_favorite(&id);
                self.state.rebuild_visible_tracks();
            }
            LibraryAction::RemoveFavorite(id) => {
                if let Err(error) = self.repository.remove_favorite(&id) {
                    eprintln!("[WARN] Failed to remove favorite: {error}");
                    return;
                }
                self.state.catalog_mut().remove_favorite(&id);
                self.state.rebuild_visible_tracks();
            }
            LibraryAction::ScanDirectory(path) => {
                self.state.set_music_directory(path.clone());
                cx.update_global::<Config, _>(|config, _cx| {
                    config.media_file.music_directory = SharedString::new(path.clone());
                });

                if let Err(error) = self.repository.scan_directory(&path) {
                    eprintln!("[WARN] Failed to scan music directory: {error}");
                }
                self.state
                    .replace_catalog(Self::load_catalog(self.repository.as_ref()));
                cx.emit(AppEvent::SettingsChanged);
            }
        }

        self.sync_playlist();
        cx.emit(AppEvent::LibraryChanged);
    }

    fn sync_playlist(&mut self) {
        if self.state.route() != AppRoute::Settings {
            self.player.set_playlist(self.state.visible_tracks());
        }
    }

    fn sync_playback(&mut self) -> bool {
        self.state.set_playback(PlaybackViewState {
            current_track: self.player.current_track().cloned(),
            play_state: self.player.play_state(),
            loop_mode: self.player.loop_mode(),
            progress: self.player.progress(),
        })
    }

    fn load_catalog(repository: &dyn LibraryRepository) -> LibraryCatalog {
        LibraryCatalog::new(
            repository.load_library(),
            repository.load_favorite_ids(),
            repository.load_history_ids(),
        )
    }
}

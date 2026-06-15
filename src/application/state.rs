use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use uuid::Uuid;

use crate::{
    audio::playlist::{LoopMode, PlayProgress, PlayState},
    db::AlbumInfo,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AppRoute {
    #[default]
    Library,
    Favorite,
    History,
    Settings,
}

#[derive(Clone, Debug)]
pub struct PlaybackViewState {
    pub current_track: Option<AlbumInfo>,
    pub play_state: PlayState,
    pub loop_mode: LoopMode,
    pub progress: Option<PlayProgress>,
}

impl Default for PlaybackViewState {
    fn default() -> Self {
        Self {
            current_track: None,
            play_state: PlayState::Stopped,
            loop_mode: LoopMode::List,
            progress: None,
        }
    }
}

impl PlaybackViewState {
    pub fn differs_from(&self, other: &Self) -> bool {
        self.current_track.as_ref().map(AlbumInfo::id)
            != other.current_track.as_ref().map(AlbumInfo::id)
            || self.play_state != other.play_state
            || self.loop_mode != other.loop_mode
            || self.progress.as_ref().map(|progress| {
                (
                    progress.elapsed,
                    progress.duration,
                    progress.progress.to_bits(),
                )
            }) != other.progress.as_ref().map(|progress| {
                (
                    progress.elapsed,
                    progress.duration,
                    progress.progress.to_bits(),
                )
            })
    }
}

pub struct LibraryCatalog {
    library: Arc<Vec<AlbumInfo>>,
    library_index: HashMap<Uuid, usize>,
    favorites: Arc<Vec<AlbumInfo>>,
    favorite_ids: HashSet<Uuid>,
    history: Arc<Vec<AlbumInfo>>,
    history_ids: HashSet<Uuid>,
}

impl LibraryCatalog {
    pub fn new(library: Vec<AlbumInfo>, favorite_ids: Vec<Uuid>, history_ids: Vec<Uuid>) -> Self {
        let library_index: HashMap<Uuid, usize> = library
            .iter()
            .enumerate()
            .map(|(index, item)| (item.id(), index))
            .collect();

        let favorite_id_order = favorite_ids;
        let history_id_order = history_ids;
        let favorite_ids: HashSet<Uuid> = favorite_id_order.iter().copied().collect();
        let history_ids: HashSet<Uuid> = history_id_order.iter().copied().collect();
        let favorites = favorite_id_order
            .iter()
            .filter_map(|id| library_index.get(id).map(|index| library[*index].clone()))
            .collect();
        let history = history_id_order
            .iter()
            .filter_map(|id| library_index.get(id).map(|index| library[*index].clone()))
            .collect();

        Self {
            library: Arc::new(library),
            library_index,
            favorites: Arc::new(favorites),
            favorite_ids,
            history: Arc::new(history),
            history_ids,
        }
    }

    pub fn library(&self) -> Arc<Vec<AlbumInfo>> {
        Arc::clone(&self.library)
    }

    pub fn favorites(&self) -> Arc<Vec<AlbumInfo>> {
        Arc::clone(&self.favorites)
    }

    pub fn history(&self) -> Arc<Vec<AlbumInfo>> {
        Arc::clone(&self.history)
    }

    pub fn get(&self, id: &Uuid) -> Option<&AlbumInfo> {
        self.library_index
            .get(id)
            .and_then(|index| self.library.get(*index))
    }

    pub fn is_favorite(&self, id: &Uuid) -> bool {
        self.favorite_ids.contains(id)
    }

    pub fn add_favorite(&mut self, id: &Uuid) -> bool {
        if !self.favorite_ids.insert(*id) {
            return false;
        }

        let Some(item) = self.get(id).cloned() else {
            self.favorite_ids.remove(id);
            return false;
        };
        let mut favorites = (*self.favorites).clone();
        favorites.push(item);
        self.favorites = Arc::new(favorites);
        true
    }

    pub fn remove_favorite(&mut self, id: &Uuid) -> bool {
        if !self.favorite_ids.remove(id) {
            return false;
        }
        self.favorites = Arc::new(
            self.favorites
                .iter()
                .filter(|item| item.id() != *id)
                .cloned()
                .collect(),
        );
        true
    }

    pub fn add_history(&mut self, id: &Uuid) -> bool {
        let Some(item) = self.get(id).cloned() else {
            return false;
        };

        let mut history: Vec<AlbumInfo> = self
            .history
            .iter()
            .filter(|entry| entry.id() != *id)
            .cloned()
            .collect();
        history.insert(0, item);
        history.truncate(100);
        self.history_ids = history.iter().map(AlbumInfo::id).collect();
        self.history = Arc::new(history);
        true
    }
}

pub struct AppState {
    route: AppRoute,
    search_query: String,
    player_details_open: bool,
    music_directory: String,
    catalog: LibraryCatalog,
    visible_tracks: Arc<Vec<AlbumInfo>>,
    playback: PlaybackViewState,
}

impl AppState {
    pub fn new(catalog: LibraryCatalog, music_directory: String) -> Self {
        let visible_tracks = catalog.library();
        Self {
            route: AppRoute::Library,
            search_query: String::new(),
            player_details_open: false,
            music_directory,
            catalog,
            visible_tracks,
            playback: PlaybackViewState::default(),
        }
    }

    pub fn route(&self) -> AppRoute {
        self.route
    }

    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    pub fn is_searching(&self) -> bool {
        !self.search_query.is_empty()
    }

    pub fn player_details_open(&self) -> bool {
        self.player_details_open
    }

    pub fn music_directory(&self) -> &str {
        &self.music_directory
    }

    pub fn visible_tracks(&self) -> Arc<Vec<AlbumInfo>> {
        Arc::clone(&self.visible_tracks)
    }

    pub fn playback(&self) -> &PlaybackViewState {
        &self.playback
    }

    pub fn is_favorite(&self, id: &Uuid) -> bool {
        self.catalog.is_favorite(id)
    }

    pub fn track(&self, id: &Uuid) -> Option<&AlbumInfo> {
        self.catalog.get(id)
    }

    pub(crate) fn set_route(&mut self, route: AppRoute) {
        self.route = route;
        self.search_query.clear();
        self.rebuild_visible_tracks();
    }

    pub(crate) fn set_search_query(&mut self, query: String) {
        self.route = AppRoute::Library;
        self.search_query = query;
        self.rebuild_visible_tracks();
    }

    pub(crate) fn set_player_details_open(&mut self, open: bool) {
        self.player_details_open = open;
    }

    pub(crate) fn set_music_directory(&mut self, directory: String) {
        self.music_directory = directory;
    }

    pub(crate) fn replace_catalog(&mut self, catalog: LibraryCatalog) {
        self.catalog = catalog;
        self.rebuild_visible_tracks();
    }

    pub(crate) fn catalog_mut(&mut self) -> &mut LibraryCatalog {
        &mut self.catalog
    }

    pub(crate) fn set_playback(&mut self, playback: PlaybackViewState) -> bool {
        let changed = playback.differs_from(&self.playback);
        self.playback = playback;
        changed
    }

    pub(crate) fn rebuild_visible_tracks(&mut self) {
        if !self.search_query.is_empty() {
            let query = self.search_query.to_lowercase();
            self.visible_tracks = Arc::new(
                self.catalog
                    .library()
                    .iter()
                    .filter(|item| {
                        item.title().to_lowercase().contains(&query)
                            || item.artist().to_lowercase().contains(&query)
                            || item.album().to_lowercase().contains(&query)
                    })
                    .cloned()
                    .collect(),
            );
            return;
        }

        self.visible_tracks = match self.route {
            AppRoute::Library => self.catalog.library(),
            AppRoute::Favorite => self.catalog.favorites(),
            AppRoute::History => self.catalog.history(),
            AppRoute::Settings => Arc::new(Vec::new()),
        };
    }
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc};

    use gpui::SharedString;
    use uuid::Uuid;

    use super::{AppRoute, AppState, LibraryCatalog};
    use crate::db::AlbumInfo;

    fn album(title: &str, artist: &str, album: &str) -> AlbumInfo {
        AlbumInfo::new(
            Uuid::new_v4(),
            SharedString::new(title),
            SharedString::new(artist),
            SharedString::new(album),
            180,
            Arc::new(PathBuf::from(format!("{title}.flac"))),
            None,
            None,
        )
    }

    #[test]
    fn search_and_navigation_rebuild_visible_tracks() {
        let first = album("First Song", "Alice", "Blue");
        let second = album("Second Song", "Bob", "Red");
        let favorite_id = second.id();
        let catalog = LibraryCatalog::new(vec![first, second], vec![favorite_id], Vec::new());
        let mut state = AppState::new(catalog, String::new());

        state.set_search_query("alice".to_string());
        assert_eq!(state.visible_tracks().len(), 1);
        assert_eq!(state.visible_tracks()[0].artist(), "Alice");

        state.set_route(AppRoute::Favorite);
        assert_eq!(state.visible_tracks().len(), 1);
        assert_eq!(state.visible_tracks()[0].id(), favorite_id);
        assert!(state.search_query().is_empty());
    }

    #[test]
    fn favorites_and_history_are_updated_in_memory() {
        let first = album("First Song", "Alice", "Blue");
        let second = album("Second Song", "Bob", "Red");
        let first_id = first.id();
        let second_id = second.id();
        let mut catalog = LibraryCatalog::new(vec![first, second], Vec::new(), Vec::new());

        assert!(catalog.add_favorite(&first_id));
        assert!(catalog.is_favorite(&first_id));
        assert!(catalog.remove_favorite(&first_id));
        assert!(!catalog.is_favorite(&first_id));

        assert!(catalog.add_history(&first_id));
        assert!(catalog.add_history(&second_id));
        assert_eq!(catalog.history()[0].id(), second_id);
        assert_eq!(catalog.history()[1].id(), first_id);
    }
}

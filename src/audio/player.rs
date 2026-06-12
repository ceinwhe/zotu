use gpui::Global;

use std::sync::Arc;

use crate::db::metadata::AlbumInfo;

use super::engine::{AudioEngine, EngineEvent};
use super::playlist::{LoopMode, PlayList, PlayProgress, PlayState};

pub struct Player {
    engine: Box<dyn AudioEngine>,
    playlist: Option<PlayList>,
    current_index: Option<usize>,
    current_shuffle_index: Option<usize>,
    current_track: Option<AlbumInfo>,
    loop_mode: LoopMode,
    play_state: PlayState,

    play_history: Vec<usize>,
    history_position: Option<usize>,

    track_start_time: Option<std::time::Instant>,
    paused_elapsed: Option<u64>,
}

impl Global for Player {}

impl Player {
    pub fn new(engine: Box<dyn AudioEngine>) -> Self {
        Self {
            engine,
            playlist: None,
            current_index: None,
            current_shuffle_index: None,
            current_track: None,
            loop_mode: LoopMode::List,
            play_state: PlayState::Stopped,
            play_history: Vec::new(),
            history_position: None,
            track_start_time: None,
            paused_elapsed: None,
        }
    }

    // ========== 播放控制 ==========

    fn play(&mut self) {
        self.engine.resume();
        self.play_state = PlayState::Play;
        if self.track_start_time.is_none() {
            self.track_start_time = Some(std::time::Instant::now());
        }
    }

    fn pause(&mut self) {
        self.engine.pause();
        self.play_state = PlayState::Paused;
        self.paused_elapsed = Some(self.current_elapsed());
    }

    fn stop(&mut self) {
        self.engine.stop();
        self.play_state = PlayState::Stopped;
        self.track_start_time = None;
        self.paused_elapsed = None;
    }

    pub fn clear(&mut self) {
        self.stop();
        self.current_track = None;
        self.current_index = None;
        self.current_shuffle_index = None;
        self.play_history.clear();
        self.history_position = None;
    }

    pub fn toggle_play(&mut self) {
        if self.current_track.is_none() {
            return;
        }
        if self.engine.is_paused() {
            self.play();
        } else {
            self.pause();
        }
    }

    // ========== 播放进度 ==========

    pub fn progress(&self) -> Option<PlayProgress> {
        let track = self.current_track.as_ref()?;
        let duration = track.duration();
        let elapsed = self.current_elapsed();

        let progress = if duration > 0 {
            elapsed as f32 / duration as f32
        } else {
            0.0
        };

        Some(PlayProgress {
            elapsed,
            duration,
            progress: progress.clamp(0.0, 1.0),
        })
    }

    fn current_elapsed(&self) -> u64 {
        match self.play_state {
            PlayState::Play => {
                if let Some(start) = self.track_start_time {
                    let base = self.paused_elapsed.unwrap_or(0);
                    let extra = start.elapsed().as_secs();
                    base + extra
                } else {
                    self.paused_elapsed.unwrap_or(0)
                }
            }
            PlayState::Paused => self.paused_elapsed.unwrap_or(0),
            PlayState::Stopped => 0,
        }
    }

    pub fn seek(&mut self, position_secs: u64) {
        if let Some(track) = self.current_track.clone() {
            if self.engine.seek(position_secs).is_ok() {
                self.paused_elapsed = Some(position_secs);
                self.track_start_time = Some(std::time::Instant::now());
            }
        }
    }

    // ========== 状态查询 ==========

    pub fn is_playing(&self) -> bool {
        self.play_state == PlayState::Play
    }

    pub fn is_paused(&self) -> bool {
        self.play_state == PlayState::Paused
    }

    pub fn play_state(&self) -> PlayState {
        self.play_state
    }

    pub fn current_track(&self) -> Option<&AlbumInfo> {
        self.current_track.as_ref()
    }

    pub fn loop_mode(&self) -> LoopMode {
        self.loop_mode
    }

    pub fn check_and_auto_next(&mut self) {
        let events = self.engine.poll_events();
        for event in events {
            match event {
                EngineEvent::TrackFinished => {
                    if self.play_state == PlayState::Play {
                        self.auto_next();
                    }
                }
                EngineEvent::Error(e) => {
                    eprintln!("[ERROR] Engine error: {}", e);
                }
            }
        }
    }

    // ========== 播放列表管理 ==========

    pub fn set_playlist(&mut self, items: Arc<Vec<AlbumInfo>>) {
        let mut playlist = PlayList::new(items);
        if self.loop_mode == LoopMode::Random {
            playlist.shuffle();
        }
        self.playlist = Some(playlist);
    }

    pub fn has_playlist(&self) -> bool {
        self.playlist.is_some()
    }

    pub fn playlist_len(&self) -> usize {
        self.playlist.as_ref().map(|p| p.len()).unwrap_or(0)
    }

    // ========== 循环模式 ==========

    pub fn set_loop_mode(&mut self, mode: LoopMode) {
        self.loop_mode = mode;
        if mode == LoopMode::Random {
            if let Some(playlist) = &mut self.playlist {
                playlist.shuffle();
                if let Some(current_idx) = self.current_index {
                    self.current_shuffle_index = playlist
                        .shuffle_order
                        .iter()
                        .position(|&i| i == current_idx);
                }
            }
        }
    }

    pub fn toggle_loop_mode(&mut self) {
        let next_mode = self.loop_mode.next();
        self.set_loop_mode(next_mode);
    }

    // ========== 播放操作 ==========

    pub fn play_track(&mut self, item: &AlbumInfo) {
        if let Some(playlist) = &self.playlist {
            if let Some(&idx) = playlist.index.get(&item.id()) {
                self.current_index = Some(idx);
                if self.loop_mode == LoopMode::Random {
                    self.current_shuffle_index =
                        playlist.shuffle_order.iter().position(|&i| i == idx);
                }
                self.add_to_history(idx);
            }
        }

        self.play_source(item);
    }

    fn add_to_history(&mut self, idx: usize) {
        if let Some(pos) = self.history_position {
            if pos < self.play_history.len().saturating_sub(1) {
                self.play_history.truncate(pos + 1);
            }
        }

        self.play_history.push(idx);
        self.history_position = Some(self.play_history.len() - 1);
    }

    pub fn can_go_back(&self) -> bool {
        match self.history_position {
            Some(pos) => pos > 0,
            None => false,
        }
    }

    pub fn next(&mut self) {
        if let Some(pos) = self.history_position {
            if pos < self.play_history.len().saturating_sub(1) {
                let next_pos = pos + 1;
                self.history_position = Some(next_pos);
                if let Some(&idx) = self.play_history.get(next_pos) {
                    self.play_by_index(idx);
                    return;
                }
            }
        }

        if let Some(playlist) = &self.playlist {
            if playlist.len() == 0 {
                return;
            }

            let next_idx = match self.loop_mode {
                LoopMode::Single => self.current_index,
                LoopMode::List => {
                    let next = self
                        .current_index
                        .map(|i| (i + 1) % playlist.len())
                        .unwrap_or(0);
                    Some(next)
                }
                LoopMode::Random => {
                    let next_shuffle_idx = self
                        .current_shuffle_index
                        .map(|i| (i + 1) % playlist.len())
                        .unwrap_or(0);
                    self.current_shuffle_index = Some(next_shuffle_idx);
                    playlist.shuffle_order.get(next_shuffle_idx).copied()
                }
            };

            if let Some(idx) = next_idx {
                self.current_index = Some(idx);
                self.add_to_history(idx);
                self.play_by_index(idx);
            }
        }
    }

    pub fn previous(&mut self) {
        if let Some(pos) = self.history_position {
            if pos > 0 {
                let prev_pos = pos - 1;
                self.history_position = Some(prev_pos);
                if let Some(&idx) = self.play_history.get(prev_pos) {
                    self.play_by_index(idx);
                    return;
                }
            }
        }

        if let Some(playlist) = &self.playlist {
            if playlist.len() == 0 {
                return;
            }

            let prev_idx = match self.loop_mode {
                LoopMode::Single => self.current_index,
                LoopMode::List => {
                    let prev = self
                        .current_index
                        .map(|i| if i == 0 { playlist.len() - 1 } else { i - 1 })
                        .unwrap_or(0);
                    Some(prev)
                }
                LoopMode::Random => {
                    let prev_shuffle_idx = self
                        .current_shuffle_index
                        .map(|i| if i == 0 { playlist.len() - 1 } else { i - 1 })
                        .unwrap_or(0);
                    self.current_shuffle_index = Some(prev_shuffle_idx);
                    playlist.shuffle_order.get(prev_shuffle_idx).copied()
                }
            };

            if let Some(idx) = prev_idx {
                self.current_index = Some(idx);
                if self.play_history.is_empty() {
                    self.add_to_history(idx);
                }
                self.play_by_index(idx);
            }
        }
    }

    // ========== 内部方法 ==========

    fn play_by_index(&mut self, idx: usize) {
        if let Some(playlist) = &self.playlist {
            if let Some(item) = playlist.get(idx) {
                self.current_index = Some(idx);
                if self.loop_mode == LoopMode::Random {
                    self.current_shuffle_index =
                        playlist.shuffle_order.iter().position(|&i| i == idx);
                }

                let item_clone = item.clone();
                drop(playlist);
                self.play_source(&item_clone);
            }
        }
    }

    fn auto_next(&mut self) {
        match self.loop_mode {
            LoopMode::Single => {
                if let Some(track) = &self.current_track {
                    let track_clone = track.clone();
                    self.play_source(&track_clone);
                }
            }
            _ => {
                self.next_auto();
            }
        }
    }

    fn next_auto(&mut self) {
        if let Some(playlist) = &self.playlist {
            if playlist.len() == 0 {
                return;
            }

            let next_idx = match self.loop_mode {
                LoopMode::Single => self.current_index,
                LoopMode::List => {
                    let next = self
                        .current_index
                        .map(|i| (i + 1) % playlist.len())
                        .unwrap_or(0);
                    Some(next)
                }
                LoopMode::Random => {
                    let next_shuffle_idx = self
                        .current_shuffle_index
                        .map(|i| (i + 1) % playlist.len())
                        .unwrap_or(0);
                    self.current_shuffle_index = Some(next_shuffle_idx);
                    playlist.shuffle_order.get(next_shuffle_idx).copied()
                }
            };

            if let Some(idx) = next_idx {
                self.current_index = Some(idx);
                self.add_to_history(idx);
                self.play_by_index(idx);
            }
        }
    }

    fn play_source(&mut self, item: &AlbumInfo) {
        let path = item.path();
        match self.engine.play(&path) {
            Ok(()) => {
                self.current_track = Some(item.clone());
                self.paused_elapsed = None;
                self.track_start_time = Some(std::time::Instant::now());
                self.play_state = PlayState::Play;
            }
            Err(e) => {
                eprintln!("[ERROR] Failed to play {:?}: {}", path, e);
                self.current_track = None;
                self.play_state = PlayState::Stopped;
            }
        }
    }
}

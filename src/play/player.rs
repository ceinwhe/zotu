use crate::db::metadata::AlbumInfo;
use gpui::{Global, SharedString};
use rand::seq::SliceRandom;
use rodio::{Decoder, OutputStream, Sink};
use uuid::Uuid;

use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;

/// 循环播放模式
#[derive(Clone, Copy, PartialEq)]
pub enum LoopMode {
    Random,
    Single,
    List,
}

impl LoopMode {
    ///点击切换到下一个循环模式
    pub fn next(&self) -> Self {
        match self {
            LoopMode::List => LoopMode::Single,
            LoopMode::Single => LoopMode::Random,
            LoopMode::Random => LoopMode::List,
        }
    }
}

/// 播放状态
#[derive(Clone, Copy, PartialEq)]
pub enum PlayState {
    Play,
    Paused,
}

/// 当前播放信息，用于 UI 显示
#[derive(Clone)]
pub struct CurrentTrackInfo {
    pub title: SharedString,
    pub artist: SharedString,
    pub album: SharedString,
    pub duration: u64,
    pub id: Uuid,
}

impl CurrentTrackInfo {
    pub fn from_album_info(info: &AlbumInfo) -> Self {
        Self {
            title: info.title(),
            artist: info.artist(),
            album: info.album(),
            duration: info.duration(),
            id: info.id,
        }
    }
}

struct PlayList {
    items: Arc<Vec<AlbumInfo>>,
    index: HashMap<Uuid, usize>,
    /// 随机播放时的播放顺序
    shuffle_order: Vec<usize>,
}

impl PlayList {
    fn new(items: Arc<Vec<AlbumInfo>>) -> Self {
        let index = items
            .iter()
            .enumerate()
            .map(|(i, item)| (item.id, i))
            .collect();
        let shuffle_order = (0..items.len()).collect();
        Self {
            items,
            index,
            shuffle_order,
        }
    }

    fn shuffle(&mut self) {
        let mut rng = rand::rng();
        self.shuffle_order.shuffle(&mut rng);
    }

    fn len(&self) -> usize {
        self.items.len()
    }

    fn get(&self, index: usize) -> Option<&AlbumInfo> {
        self.items.get(index)
    }

}

pub struct Player {
    _stream: OutputStream,
    sink: Sink,
    playlist: Option<PlayList>,
    current_index: Option<usize>,
    current_shuffle_index: Option<usize>,
    current_track: Option<CurrentTrackInfo>,
    loop_mode: LoopMode,
    play_state: PlayState,
    /// 播放历史记录 (存储播放过的歌曲索引)
    play_history: Vec<usize>,
    /// 当前在历史记录中的位置
    history_position: Option<usize>,
}

impl Global for Player {}

impl Player {
    pub fn new() -> Self {
        let stream =
            rodio::OutputStreamBuilder::open_default_stream().expect("open default audio stream");

        let sink = Sink::connect_new(&stream.mixer());

        Self {
            _stream: stream,
            sink,
            playlist: None,
            current_index: None,
            current_shuffle_index: None,
            current_track: None,
            loop_mode: LoopMode::List,
            play_state: PlayState::Paused,
            play_history: Vec::new(),
            history_position: None,
        }
    }

    // ========== 播放控制 ==========

    pub fn play(&mut self) {
        self.sink.play();
        if self.current_track.is_some() {
            self.play_state = PlayState::Play;
        }
    }

    pub fn pause(&mut self) {
        self.sink.pause();
        self.play_state = PlayState::Paused;
    }

    /// 停止播放并清空播放状态
    pub fn clear(&mut self) {
        self.sink.stop();
        self.play_state = PlayState::Paused;
        self.current_track = None;
        self.current_index = None;
        self.current_shuffle_index = None;
        self.play_history.clear();
        self.history_position = None;
    }

    pub fn toggle_play(&mut self) {
        // 如果没有歌曲，不做任何操作
        if self.current_track.is_none() {
            return;
        }
        if self.sink.is_paused() {
            self.play();
        } else {
            self.pause();
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

    pub fn current_track(&self) -> Option<&CurrentTrackInfo> {
        self.current_track.as_ref()
    }

    pub fn loop_mode(&self) -> LoopMode {
        self.loop_mode
    }

    /// 检查 sink 是否播放完毕，用于轮询自动下一首
    pub fn check_and_auto_next(&mut self) {
        if self.sink.empty() && self.play_state == PlayState::Play {
            self.auto_next();
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
        // 切换到随机模式时重新洗牌
        if mode == LoopMode::Random {
            if let Some(playlist) = &mut self.playlist {
                playlist.shuffle();
                // 如果当前有播放，找到当前歌曲在随机序列中的位置
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

    /// 点击歌曲列表中的歌曲播放
    pub fn play_track(&mut self, item: &AlbumInfo) {
        // 查找在播放列表中的索引
        if let Some(playlist) = &self.playlist {
            if let Some(&idx) = playlist.index.get(&item.id) {
                self.current_index = Some(idx);
                // 更新随机播放索引
                if self.loop_mode == LoopMode::Random {
                    self.current_shuffle_index =
                        playlist.shuffle_order.iter().position(|&i| i == idx);
                }

                // 添加到播放历史
                self.add_to_history(idx);
            }
        }

        self.play_source(item);
    }

    /// 添加索引到播放历史
    fn add_to_history(&mut self, idx: usize) {
        // 如果当前不在历史末尾，截断后面的历史
        if let Some(pos) = self.history_position {
            if pos < self.play_history.len().saturating_sub(1) {
                self.play_history.truncate(pos + 1);
            }
        }

        // 添加新的历史记录
        self.play_history.push(idx);
        self.history_position = Some(self.play_history.len() - 1);
    }

    /// 检查是否可以返回上一首（历史中有记录）
    pub fn can_go_back(&self) -> bool {
        match self.history_position {
            Some(pos) => pos > 0,
            None => false,
        }
    }

    /// 播放下一首
    pub fn next(&mut self) {
        // 如果在历史中间，先检查是否可以前进
        if let Some(pos) = self.history_position {
            if pos < self.play_history.len().saturating_sub(1) {
                // 可以在历史中前进
                let next_pos = pos + 1;
                self.history_position = Some(next_pos);
                if let Some(&idx) = self.play_history.get(next_pos) {
                    self.play_by_index(idx);
                    return;
                }
            }
        }

        // 否则按照播放模式播放下一首
        if let Some(playlist) = &self.playlist {
            if playlist.len() == 0 {
                return;
            }

            let next_idx = match self.loop_mode {
                LoopMode::Single => {
                    // 单曲循环：重新播放当前歌曲
                    self.current_index
                }
                LoopMode::List => {
                    // 列表循环：播放下一首
                    let next = self
                        .current_index
                        .map(|i| (i + 1) % playlist.len())
                        .unwrap_or(0);
                    Some(next)
                }
                LoopMode::Random => {
                    // 随机播放：按随机顺序播放下一首
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

    /// 播放上一首
    pub fn previous(&mut self) {
        // 优先从历史记录中返回
        if let Some(pos) = self.history_position {
            if pos > 0 {
                // 可以在历史中后退
                let prev_pos = pos - 1;
                self.history_position = Some(prev_pos);
                if let Some(&idx) = self.play_history.get(prev_pos) {
                    self.play_by_index(idx);
                    return;
                }
            }
        }

        // 如果没有历史或已经在历史开头，按照播放模式播放上一首
        if let Some(playlist) = &self.playlist {
            if playlist.len() == 0 {
                return;
            }

            let prev_idx = match self.loop_mode {
                LoopMode::Single => {
                    // 单曲循环：重新播放当前歌曲
                    self.current_index
                }
                LoopMode::List => {
                    // 列表循环：播放上一首
                    let prev = self
                        .current_index
                        .map(|i| if i == 0 { playlist.len() - 1 } else { i - 1 })
                        .unwrap_or(0);
                    Some(prev)
                }
                LoopMode::Random => {
                    // 随机播放：按随机顺序播放上一首
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
                // 添加到历史（如果历史为空的话）
                if self.play_history.is_empty() {
                    self.add_to_history(idx);
                }
                self.play_by_index(idx);
            }
        }
    }

    /// 根据索引播放歌曲（内部使用）
    fn play_by_index(&mut self, idx: usize) {
        if let Some(playlist) = &self.playlist {
            if let Some(item) = playlist.get(idx) {
                self.current_index = Some(idx);
                // 更新随机播放索引
                if self.loop_mode == LoopMode::Random {
                    self.current_shuffle_index =
                        playlist.shuffle_order.iter().position(|&i| i == idx);
                }
                let item_clone = CurrentTrackInfo::from_album_info(item);
                let path = item.path();
                self.play_source_by_path(path, item_clone);
            }
        }
    }

    /// 自动播放下一首（歌曲播放完毕时调用）
    fn auto_next(&mut self) {
        match self.loop_mode {
            LoopMode::Single => {
                // 单曲循环：重新播放当前歌曲
                if let Some(track) = &self.current_track {
                    if let Some(playlist) = &self.playlist {
                        if let Some(&idx) = playlist.index.get(&track.id) {
                            if let Some(item) = playlist.get(idx) {
                                let path = item.path();
                                if let Ok(source) = decode(path) {
                                    self.sink.append(source);
                                }
                            }
                        }
                    }
                }
            }
            _ => {
                // 列表循环和随机播放：播放下一首（自动播放时总是往前走，添加到历史）
                self.next_auto();
            }
        }
    }

    /// 自动播放下一首（不检查历史，总是按顺序/随机播放下一首）
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

    // ========== 内部方法 ==========

    fn play_source(&mut self, item: &AlbumInfo) {
        let path = item.path();
        let track_info = CurrentTrackInfo::from_album_info(item);
        self.play_source_by_path(path, track_info);
    }

    fn play_source_by_path(&mut self, path: Arc<PathBuf>, track_info: CurrentTrackInfo) {
        // 停止当前播放
        self.sink.stop();
        // 重新创建 sink
        self.sink = Sink::connect_new(&self._stream.mixer());

        if let Ok(source) = decode(path) {
            self.sink.append(source);
            self.current_track = Some(track_info);
            self.play_state = PlayState::Play;
        }
    }
}

/// 解码音频文件
fn decode(
    path: Arc<PathBuf>,
) -> Result<Decoder<std::io::BufReader<std::fs::File>>, Box<dyn Error + Send + Sync>> {
    let file = std::fs::File::open(path.as_path())?;
    let source = Decoder::new(std::io::BufReader::new(file))?;
    Ok(source)
}

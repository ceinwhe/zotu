use gpui::Global;
use rand::seq::SliceRandom;
use rodio::{Decoder, OutputStream, Sink};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use std::{collections::HashMap, error::Error, path::PathBuf, sync::Arc, time::Instant};

use crate::db::metadata::AlbumInfo;

/// 循环播放模式
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
pub enum LoopMode {
    Random,
    Single,
    List,
}

impl LoopMode {
    /// 点击切换到下一个循环模式
    pub fn next(&self) -> Self {
        match self {
            LoopMode::List => LoopMode::Single,
            LoopMode::Single => LoopMode::Random,
            LoopMode::Random => LoopMode::List,
        }
    }
}

/// 播放状态
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PlayState {
    Play,
    Paused,
    Stopped,
}

/// 播放进度信息（供 UI 使用）
#[derive(Clone, Debug)]
pub struct PlayProgress {
    /// 当前播放位置（秒）
    pub elapsed: u64,
    /// 总时长（秒）
    pub duration: u64,
    /// 播放进度比例 (0.0 ~ 1.0)
    pub progress: f32,
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
            .map(|(i, item)| (item.id(), i))
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
    stream: OutputStream,
    sink: Sink,
    playlist: Option<PlayList>,
    current_index: Option<usize>,
    current_shuffle_index: Option<usize>,
    current_track: Option<AlbumInfo>,
    loop_mode: LoopMode,
    play_state: PlayState,

    /// 播放历史记录（存储播放过的歌曲索引）
    play_history: Vec<usize>,
    /// 当前在历史记录中的位置
    history_position: Option<usize>,

    /// 当前曲目开始播放的时间点（用于计算进度）
    track_start_time: Option<Instant>,
    /// 暂停时已播放的时长（秒）
    paused_elapsed: Option<u64>,
}

impl Global for Player {}

impl Player {
    pub fn new() -> Self {
        let stream =
            rodio::OutputStreamBuilder::open_default_stream().expect("open default audio stream");

        let sink = Sink::connect_new(&stream.mixer());

        Self {
            stream,
            sink,
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
        self.sink.play();
        self.play_state = PlayState::Play;
        // 记录开始播放的时间点
        if self.track_start_time.is_none() {
            self.track_start_time = Some(Instant::now());
        }
    }

    fn pause(&mut self) {
        self.sink.pause();
        self.play_state = PlayState::Paused;
        // 保存暂停时已播放的时长
        self.paused_elapsed = Some(self.current_elapsed());
    }

    fn stop(&mut self) {
        self.sink.stop();
        self.play_state = PlayState::Stopped;
        self.track_start_time = None;
        self.paused_elapsed = None;
    }

    /// 停止播放并清空播放状态
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
        if self.sink.is_paused() {
            self.play();
        } else {
            self.pause();
        }
    }

    // ========== 播放进度 ==========

    /// 获取当前播放进度
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

    /// 计算当前已播放时长（秒）
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

    /// Seek 到指定位置（秒）
    pub fn seek(&mut self, position_secs: u64) {
        if let Some(track) = self.current_track.clone() {
            let path = track.path();
            // 保存当前位置信息
            self.paused_elapsed = Some(position_secs);
            // 重新开始播放
            self.play_source_internal(path, track, Some(position_secs));
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

    /// 点击歌曲列表中的歌曲播放
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

        self.play_source(item, None);
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

    /// 播放下一首
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

    /// 播放上一首
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

                let path = item.path();
                self.play_source_internal(path, item.clone(), None);
            }
        }
    }

    /// 自动播放下一首（由后台线程调用）
    fn auto_next(&mut self) {
        match self.loop_mode {
            LoopMode::Single => {
                if let Some(track) = &self.current_track {
                    let path = track.path();
                    self.play_source_internal(path, track.clone(), None);
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

    fn play_source(&mut self, item: &AlbumInfo, seek_to: Option<u64>) {
        let path = item.path();
        self.play_source_internal(path, item.clone(), seek_to);
    }

    fn play_source_internal(
        &mut self,
        path: Arc<PathBuf>,
        track_info: AlbumInfo,
        seek_to: Option<u64>,
    ) {
        // 停止当前播放
        self.sink.stop();

        // 重新创建 sink
        self.sink = Sink::connect_new(&self.stream.mixer());

        match decode(path.clone()) {
            Ok(source) => {
                self.sink.append(source);
                self.current_track = Some(track_info);

                // 如果有 seek 位置，则跳转（注意：rodio 的 Sink 不支持 seek，这是一个近似实现）
                if let Some(_pos) = seek_to {
                    // rodio 0.21 的 Sink 不支持精确 seek
                    // 作为近似，记录起始偏移
                    self.paused_elapsed = seek_to;
                } else {
                    self.paused_elapsed = None;
                }

                self.track_start_time = Some(Instant::now());
                self.play_state = PlayState::Play;
                self.sink.play();
            }
            Err(e) => {
                eprintln!("[ERROR] 解码音频文件失败: {:?} - {}", path, e);
                self.current_track = None;
                self.play_state = PlayState::Stopped;
            }
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

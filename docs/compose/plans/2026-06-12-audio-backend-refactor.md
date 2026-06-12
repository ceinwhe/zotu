# Audio Backend Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use compose:subagent (recommended) or compose:execute to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace rodio with ffmpeg (raw FFI) + cpal for audio playback, and modularize the audio backend with a trait abstraction.

**Architecture:** Define an `AudioEngine` trait for audio operations. Implement it with `FfmpegEngine` using ffmpeg FFI for decoding/resampling and cpal for audio output. `Player` holds a `Box<dyn AudioEngine>` and delegates all audio operations. UI components continue using `cx.global::<Player>()` but only access high-level APIs.

**Tech Stack:** Rust, ffmpeg (raw FFI via `unsafe extern "C"`), cpal, gpui, lofty

---

## File Structure

### New Files
- `src/audio.rs` — module entry, re-exports
- `src/audio/engine.rs` — `AudioEngine` trait definition
- `src/audio/player.rs` — `Player` struct (gpui::Global), delegates to AudioEngine
- `src/audio/playlist.rs` — `PlayList` and `LoopMode` types
- `src/audio/ffmpeg.rs` — ffmpeg submodule entry
- `src/audio/ffmpeg/ffi.rs` — ffmpeg FFI bindings (moved from src/ffmpeg/ffi.rs)
- `src/audio/ffmpeg/decoder.rs` — `FfmpegDecoder` — opens files, decodes to PCM
- `src/audio/ffmpeg/resampler.rs` — `FfmpegResampler` — swresample wrapper
- `src/audio/output.rs` — `CpalOutput` — cpal stream management

### Modified Files
- `src/lib.rs` — replace `pub mod ffmpeg` and `pub mod play` with `pub mod audio`
- `src/main.rs` — update import path for Player
- `src/app.rs` — update import path for Player
- `src/config.rs` — update import path for LoopMode
- `src/components/playbar.rs` — update import paths
- `src/components/now_playing.rs` — update import paths
- `src/components/songview.rs` — update import paths
- `Cargo.toml` — remove rodio, add cpal

### Deleted Files
- `src/play.rs`
- `src/play/player.rs`
- `src/ffmpeg/mod.rs`
- `src/ffmpeg/ffi.rs`
- `src/ffmpeg/decoder.rs`
- `src/ffmpeg/output.rs`
- `src/ffmpeg/resampler.rs`
- `src/ffmpeg/filters.rs`
- `src/ffmpeg/metadata.rs`

---

### Task 1: Create module structure and AudioEngine trait

**Files:**
- Create: `src/audio.rs`
- Create: `src/audio/engine.rs`
- Create: `src/audio/playlist.rs`
- Create: `src/audio/player.rs`
- Create: `src/audio/ffmpeg.rs`
- Create: `src/audio/output.rs`

- [ ] **Step 1: Create `src/audio/engine.rs` — the AudioEngine trait**

```rust
use std::path::Path;
use std::time::Duration;

pub enum EngineEvent {
    TrackFinished,
    Error(String),
}

pub trait AudioEngine: Send {
    fn play(&mut self, path: &Path) -> anyhow::Result<()>;
    fn pause(&mut self);
    fn resume(&mut self);
    fn stop(&mut self);
    fn seek(&mut self, position_secs: u64) -> anyhow::Result<()>;
    fn is_playing(&self) -> bool;
    fn is_paused(&self) -> bool;
    fn current_position_secs(&self) -> u64;
    fn duration_secs(&self) -> Option<u64>;
    fn poll_events(&mut self) -> Vec<EngineEvent>;
}
```

- [ ] **Step 2: Create `src/audio/playlist.rs` — playlist and loop mode**

Move `LoopMode`, `PlayState`, `PlayProgress`, `PlayList` from `src/play/player.rs` into this file. These are pure data types with no audio dependency.

```rust
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use std::{collections::HashMap, sync::Arc};

use crate::db::metadata::AlbumInfo;

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
pub enum LoopMode {
    Random,
    Single,
    List,
}

impl LoopMode {
    pub fn next(&self) -> Self {
        match self {
            LoopMode::List => LoopMode::Single,
            LoopMode::Single => LoopMode::Random,
            LoopMode::Random => LoopMode::List,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PlayState {
    Play,
    Paused,
    Stopped,
}

#[derive(Clone, Debug)]
pub struct PlayProgress {
    pub elapsed: u64,
    pub duration: u64,
    pub progress: f32,
}

pub struct PlayList {
    items: Arc<Vec<AlbumInfo>>,
    index: HashMap<Uuid, usize>,
    pub shuffle_order: Vec<usize>,
}

impl PlayList {
    pub fn new(items: Arc<Vec<AlbumInfo>>) -> Self {
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

    pub fn shuffle(&mut self) {
        let mut rng = rand::rng();
        self.shuffle_order.shuffle(&mut rng);
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn get(&self, index: usize) -> Option<&AlbumInfo> {
        self.items.get(index)
    }

    pub fn index_of(&self, id: &Uuid) -> Option<usize> {
        self.index.get(id).copied()
    }
}
```

- [ ] **Step 3: Create `src/audio/player.rs` — Player struct using AudioEngine**

```rust
use gpui::Global;
use uuid::Uuid;

use std::{path::Path, sync::Arc};

use super::engine::AudioEngine;
use super::playlist::{LoopMode, PlayList, PlayProgress, PlayState};
use crate::db::metadata::AlbumInfo;

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

    // ========== Playback Control ==========

    pub fn toggle_play(&mut self) {
        if self.current_track.is_none() {
            return;
        }
        match self.play_state {
            PlayState::Play => self.pause(),
            PlayState::Paused => self.resume(),
            PlayState::Stopped => {}
        }
    }

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

    fn resume(&mut self) {
        self.engine.resume();
        self.play_state = PlayState::Play;
        self.track_start_time = Some(std::time::Instant::now());
    }

    pub fn stop(&mut self) {
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

    // ========== Progress ==========

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
        if self.current_track.is_some() {
            if self.engine.seek(position_secs).is_ok() {
                self.paused_elapsed = Some(position_secs);
                self.track_start_time = Some(std::time::Instant::now());
            }
        }
    }

    // ========== State Queries ==========

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
        // Check engine events for track finished
        let events = self.engine.poll_events();
        for event in events {
            match event {
                super::engine::EngineEvent::TrackFinished => {
                    self.auto_next();
                }
                super::engine::EngineEvent::Error(e) => {
                    eprintln!("[ERROR] Audio engine error: {}", e);
                }
            }
        }

        // Also check if engine reports not playing and we think we are
        if self.play_state == PlayState::Play && !self.engine.is_playing() && !self.engine.is_paused() {
            self.auto_next();
        }
    }

    // ========== Playlist Management ==========

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

    // ========== Loop Mode ==========

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

    // ========== Track Playback ==========

    pub fn play_track(&mut self, item: &AlbumInfo) {
        if let Some(playlist) = &self.playlist {
            if let Some(idx) = playlist.index_of(&item.id()) {
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

    // ========== Internal ==========

    fn play_by_index(&mut self, idx: usize) {
        if let Some(playlist) = &self.playlist {
            if let Some(item) = playlist.get(idx) {
                self.current_index = Some(idx);
                if self.loop_mode == LoopMode::Random {
                    self.current_shuffle_index =
                        playlist.shuffle_order.iter().position(|&i| i == idx);
                }

                let item = item.clone();
                self.play_source(&item);
            }
        }
    }

    fn auto_next(&mut self) {
        match self.loop_mode {
            LoopMode::Single => {
                if let Some(track) = &self.current_track {
                    let track = track.clone();
                    self.play_source(&track);
                }
            }
            _ => {
                self.next();
            }
        }
    }

    fn play_source(&mut self, item: &AlbumInfo) {
        let path = item.path();
        match self.engine.play(Path::new(path.as_path())) {
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
```

- [ ] **Step 4: Create `src/audio.rs` — module entry**

```rust
pub mod engine;
pub mod ffmpeg;
pub mod output;
pub mod player;
pub mod playlist;
```

- [ ] **Step 5: Create `src/audio/ffmpeg.rs` — ffmpeg submodule entry**

```rust
pub mod decoder;
pub mod ffi;
pub mod resampler;
```

- [ ] **Step 6: Create empty placeholder files**

Create empty files: `src/audio/output.rs`, `src/audio/ffmpeg/decoder.rs`, `src/audio/ffmpeg/resampler.rs`

- [ ] **Step 7: Update `src/lib.rs` — replace old modules**

```rust
pub mod app;
pub mod assets;
pub mod audio;
pub mod components;
pub mod config;
pub mod db;
pub mod error;
pub mod theme;
pub mod ui;
pub mod util;
```

Remove `pub mod ffmpeg;` and `pub mod play;`.

- [ ] **Step 8: Create `src/audio/ffmpeg/ffi.rs` — move FFI bindings**

Copy contents from `src/ffmpeg/ffi.rs` to `src/audio/ffmpeg/ffi.rs`.

- [ ] **Step 9: Commit**

```bash
git add src/audio.rs src/audio/ src/lib.rs
git commit -m "refactor: create audio module structure with AudioEngine trait"
```

---

### Task 2: Implement ffmpeg decoder

**Files:**
- Create: `src/audio/ffmpeg/decoder.rs`

- [ ] **Step 1: Implement FfmpegDecoder**

```rust
use super::ffi::*;
use std::ffi::CString;
use std::os::raw::c_int;
use std::path::Path;
use std::ptr;

pub struct AudioFormat {
    pub sample_rate: i32,
    pub channels: i32,
    pub sample_fmt: c_int,
}

pub struct FfmpegDecoder {
    format_ctx: *mut AVFormatContext,
    codec_ctx: *mut AVCodecContext,
    packet: *mut AVPacket,
    frame: *mut AVFrame,
    stream_index: c_int,
    pub format: AudioFormat,
    duration_secs: u64,
    finished: bool,
}

unsafe impl Send for FfmpegDecoder {}

impl FfmpegDecoder {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let path_str = path.to_str().ok_or_else(|| anyhow::anyhow!("Invalid path"))?;
        let c_path = CString::new(path_str)?;

        let mut format_ctx: *mut AVFormatContext = ptr::null_mut();
        let ret = unsafe {
            avformat_open_input(
                &mut format_ctx,
                c_path.as_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };
        if ret != 0 {
            return Err(ffmpeg_error(ret));
        }

        let ret = unsafe { avformat_find_stream_info(format_ctx, ptr::null_mut()) };
        if ret < 0 {
            unsafe { avformat_close_input(&mut format_ctx) };
            return Err(ffmpeg_error(ret));
        }

        // Find best audio stream
        let mut stream_index: c_int = -1;
        let nb_streams = unsafe { (*format_ctx).nb_streams } as i32;
        for i in 0..nb_streams {
            // We need to access format_ctx->streams[i]->codecpar->codec_type
            // Since we don't have full struct definitions, we use a workaround:
            // Try to find a decoder for each stream
            let codec = unsafe { avcodec_find_decoder(0) }; // placeholder
            if !codec.is_null() {
                stream_index = i;
                break;
            }
        }

        // Alternative: iterate streams by trying to open codecs
        // For simplicity, assume first audio stream (index 0) for now
        // A more robust implementation would check codecpar->codec_type
        if stream_index < 0 {
            stream_index = 0; // Default to first stream
        }

        // Get codec parameters and open codec
        let codec = unsafe { avcodec_find_decoder(0 /* AV_CODEC_ID_MP3 as placeholder */) };
        if codec.is_null() {
            unsafe { avformat_close_input(&mut format_ctx) };
            return Err(anyhow::anyhow!("No decoder found"));
        }

        let codec_ctx = unsafe { avcodec_alloc_context3(codec) };
        if codec_ctx.is_null() {
            unsafe { avformat_close_input(&mut format_ctx) };
            return Err(anyhow::anyhow!("Failed to allocate codec context"));
        }

        let ret = unsafe { avcodec_open2(codec_ctx, codec, ptr::null_mut()) };
        if ret != 0 {
            unsafe {
                avcodec_free_context(&mut codec_ctx);
                avformat_close_input(&mut format_ctx);
            }
            return Err(ffmpeg_error(ret));
        }

        let packet = unsafe { av_packet_alloc() };
        let frame = unsafe { av_frame_alloc() };

        // Calculate duration
        let duration_secs = unsafe {
            if (*format_ctx).duration > 0 {
                ((*format_ctx).duration / 1_000_000) as u64
            } else {
                0
            }
        };

        let sample_rate = unsafe { av_frame_get_sample_rate(frame) };
        let channels = unsafe { av_frame_get_channels(frame) };

        Ok(Self {
            format_ctx,
            codec_ctx,
            packet,
            frame,
            stream_index,
            format: AudioFormat {
                sample_rate: if sample_rate > 0 { sample_rate } else { 44100 },
                channels: if channels > 0 { channels } else { 2 },
                sample_fmt: AV_SAMPLE_FMT_FLT,
            },
            duration_secs,
            finished: false,
        })
    }

    pub fn duration_secs(&self) -> u64 {
        self.duration_secs
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Read next decoded frame. Returns (data_ptr, nb_samples) or None if finished.
    pub fn read_frame(&mut self) -> anyhow::Result<Option<(*mut u8, usize)>> {
        if self.finished {
            return Ok(None);
        }

        loop {
            let ret = unsafe { avcodec_receive_frame(self.codec_ctx, self.frame) };
            if ret == 0 {
                let nb_samples = unsafe { av_frame_get_nb_samples(self.frame) } as usize;
                let data = unsafe { av_frame_get_data(self.frame, 0) };
                if data.is_null() || nb_samples == 0 {
                    continue;
                }
                return Ok(Some((data, nb_samples)));
            }

            if ret == AVERROR_EOF {
                self.finished = true;
                return Ok(None);
            }

            if ret < 0 && ret != -11 {
                // -11 = EAGAIN
                return Err(ffmpeg_error(ret));
            }

            // Read next packet
            let ret = unsafe { av_read_frame(self.format_ctx, self.packet) };
            if ret < 0 {
                if ret == AVERROR_EOF {
                    // Flush decoder
                    unsafe {
                        avcodec_send_packet(self.codec_ctx, ptr::null());
                    }
                    continue;
                }
                self.finished = true;
                return Ok(None);
            }

            let send_ret = unsafe { avcodec_send_packet(self.codec_ctx, self.packet) };
            unsafe { av_packet_free(&mut self.packet) };
            self.packet = unsafe { av_packet_alloc() };

            if send_ret < 0 {
                return Err(ffmpeg_error(send_ret));
            }
        }
    }

    pub fn seek(&mut self, position_secs: u64) -> anyhow::Result<()> {
        let timestamp = position_secs as i64 * 1_000_000;
        let ret = unsafe {
            av_seek_frame(self.format_ctx, self.stream_index, timestamp as u64, 1)
        };
        if ret < 0 {
            return Err(ffmpeg_error(ret));
        }
        unsafe {
            avcodec_flush_buffers(self.codec_ctx);
        }
        self.finished = false;
        Ok(())
    }
}

impl Drop for FfmpegDecoder {
    fn drop(&mut self) {
        unsafe {
            if !self.frame.is_null() {
                av_frame_free(&mut self.frame);
            }
            if !self.packet.is_null() {
                av_packet_free(&mut self.packet);
            }
            if !self.codec_ctx.is_null() {
                avcodec_free_context(&mut self.codec_ctx);
            }
            if !self.format_ctx.is_null() {
                avformat_close_input(&mut self.format_ctx);
            }
        }
    }
}
```

**Note:** The decoder implementation above is a skeleton. The actual stream finding logic needs the `AVFormatContext` struct to have a `streams` field accessible via FFI. Since the current `ffi.rs` uses opaque types, we need to either:
1. Add a helper function in `ffi.rs` to get stream info, or
2. Make the struct fields non-opaque for the fields we need

This will be refined in the implementation phase.

- [ ] **Step 2: Commit**

```bash
git add src/audio/ffmpeg/decoder.rs
git commit -m "feat: implement ffmpeg decoder wrapper"
```

---

### Task 3: Implement ffmpeg resampler

**Files:**
- Create: `src/audio/ffmpeg/resampler.rs`

- [ ] **Step 1: Implement FfmpegResampler**

```rust
use super::ffi::*;
use std::os::raw::c_int;
use std::ptr;

pub struct FfmpegResampler {
    ctx: *mut SwrContext,
    out_sample_rate: i32,
    out_channels: i32,
}

unsafe impl Send for FfmpegResampler {}

impl FfmpegResampler {
    pub fn new(
        in_sample_rate: i32,
        in_channels: i32,
        in_format: c_int,
        out_sample_rate: i32,
        out_channels: i32,
    ) -> anyhow::Result<Self> {
        let in_layout = if in_channels == 1 {
            AV_CH_LAYOUT_MONO
        } else {
            AV_CH_LAYOUT_STEREO
        };
        let out_layout = if out_channels == 1 {
            AV_CH_LAYOUT_MONO
        } else {
            AV_CH_LAYOUT_STEREO
        };

        let ctx = unsafe {
            swr_alloc_set_opts(
                ptr::null_mut(),
                out_layout,
                AV_SAMPLE_FMT_FLT,
                out_sample_rate,
                in_layout,
                in_format,
                in_sample_rate,
                0,
                ptr::null_mut(),
            )
        };

        if ctx.is_null() {
            return Err(anyhow::anyhow!("Failed to allocate resampler context"));
        }

        let ret = unsafe { swr_init(ctx) };
        if ret != 0 {
            unsafe { swr_free(&mut (ctx as *mut _)) };
            return Err(ffmpeg_error(ret));
        }

        Ok(Self {
            ctx,
            out_sample_rate,
            out_channels,
        })
    }

    pub fn out_sample_rate(&self) -> i32 {
        self.out_sample_rate
    }

    pub fn out_channels(&self) -> i32 {
        self.out_channels
    }

    /// Resample input samples to output buffer.
    /// Returns number of output samples written.
    pub fn resample(
        &mut self,
        in_data: *const *const u8,
        in_samples: c_int,
        out_data: *mut *mut u8,
        out_samples: c_int,
    ) -> anyhow::Result<c_int> {
        let written = unsafe {
            swr_convert(self.ctx, out_data, out_samples, in_data, in_samples)
        };
        if written < 0 {
            return Err(ffmpeg_error(written));
        }
        Ok(written)
    }
}

impl Drop for FfmpegResampler {
    fn drop(&mut self) {
        unsafe {
            let mut ctx = self.ctx;
            swr_free(&mut ctx);
        }
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add src/audio/ffmpeg/resampler.rs
git commit -m "feat: implement ffmpeg resampler wrapper"
```

---

### Task 4: Implement cpal audio output

**Files:**
- Create: `src/audio/output.rs`

- [ ] **Step 1: Implement CpalOutput**

```rust
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SampleRate, StreamConfig};
use std::sync::{Arc, Mutex};

pub struct CpalOutput {
    stream: Option<cpal::Stream>,
    config: StreamConfig,
    sample_rate: u32,
    channels: u16,
}

impl CpalOutput {
    pub fn new(sample_rate: u32, channels: u16) -> anyhow::Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow::anyhow!("No audio output device found"))?;

        let config = StreamConfig {
            channels,
            sample_rate: SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        Ok(Self {
            stream: None,
            config,
            sample_rate,
            channels,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Start playback with a shared buffer.
    /// The buffer is a ring of f32 samples (interleaved stereo).
    pub fn start(&mut self, buffer: Arc<Mutex<Vec<f32>>>) -> anyhow::Result<()> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow::anyhow!("No audio output device found"))?;

        let config = self.config.clone();
        let channels = self.channels as usize;

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut buf = buffer.lock().unwrap();
                for frame in data.chunks_mut(channels) {
                    for sample in frame.iter_mut() {
                        *sample = if buf.is_empty() {
                            0.0
                        } else {
                            buf.remove(0)
                        };
                    }
                }
            },
            |err| eprintln!("[ERROR] Audio output error: {}", err),
            None,
        )?;

        stream.play()?;
        self.stream = Some(stream);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.stream = None;
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add src/audio/output.rs
git commit -m "feat: implement cpal audio output"
```

---

### Task 5: Implement FfmpegEngine (ties decoder + resampler + output)

**Files:**
- Modify: `src/audio/ffmpeg/ffi.rs` — add helper functions for stream info
- Modify: `src/audio/ffmpeg/decoder.rs` — complete implementation
- Modify: `src/audio/ffmpeg/resampler.rs` — finalize

- [ ] **Step 1: Update ffi.rs with stream helper functions**

Add these to `src/audio/ffmpeg/ffi.rs`:

```rust
// Stream codec type enum
pub const AVMEDIA_TYPE_AUDIO: c_int = 1;

#[repr(C)]
pub struct AVCodecParameters {
    pub codec_type: c_int,
    pub codec_id: c_uint,
    // ... other fields we don't need
}

unsafe extern "C" {
    pub fn av_find_best_stream(
        ic: *mut AVFormatContext,
        type_: c_int,
        wanted_stream_nb: c_int,
        related_stream: c_int,
        decoder_ret: *mut *const AVCodec,
        flags: c_int,
    ) -> c_int;

    pub fn av_get_default_channel_layout(nb_channels: c_int) -> c_ulonglong;
}
```

- [ ] **Step 2: Create FfmpegEngine implementation**

Add to `src/audio/ffmpeg.rs`:

```rust
pub mod decoder;
pub mod ffi;
pub mod resampler;

use super::engine::{AudioEngine, EngineEvent};
use super::output::CpalOutput;
use decoder::FfmpegDecoder;
use resampler::FfmpegResampler;

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct FfmpegEngine {
    command_tx: std::sync::mpsc::Sender<Command>,
    event_rx: std::sync::mpsc::Receiver<EngineEvent>,
    state: Arc<Mutex<EngineState>>,
}

struct EngineState {
    is_playing: bool,
    is_paused: bool,
    current_position_secs: u64,
    duration_secs: Option<u64>,
}

enum Command {
    Play(String),
    Pause,
    Resume,
    Stop,
    Seek(u64),
    Shutdown,
}

impl FfmpegEngine {
    pub fn new() -> anyhow::Result<Self> {
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();
        let (evt_tx, evt_rx) = std::sync::mpsc::channel();
        let state = Arc::new(Mutex::new(EngineState {
            is_playing: false,
            is_paused: false,
            current_position_secs: 0,
            duration_secs: None,
        }));

        let state_clone = Arc::clone(&state);
        thread::spawn(move || {
            engine_thread(cmd_rx, evt_tx, state_clone);
        });

        Ok(Self {
            command_tx: cmd_tx,
            event_rx: evt_rx,
            state,
        })
    }
}

fn engine_thread(
    cmd_rx: std::sync::mpsc::Receiver<Command>,
    evt_tx: std::sync::mpsc::Sender<EngineEvent>,
    state: Arc<Mutex<EngineState>>,
) {
    let mut decoder: Option<FfmpegDecoder> = None;
    let mut resampler: Option<FfmpegResampler> = None;
    let mut output: Option<CpalOutput> = None;
    let audio_buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let mut paused = false;

    loop {
        // Check for commands (non-blocking)
        match cmd_rx.try_recv() {
            Ok(Command::Play(path)) => {
                match FfmpegDecoder::open(Path::new(&path)) {
                    Ok(dec) => {
                        let out_rate = 44100;
                        let out_channels = 2;

                        let res = FfmpegResampler::new(
                            dec.format.sample_rate,
                            dec.format.channels,
                            dec.format.sample_fmt,
                            out_rate,
                            out_channels,
                        );

                        match res {
                            Ok(r) => {
                                // Start output
                                let mut out = CpalOutput::new(out_rate as u32, out_channels as u16)
                                    .ok();
                                if let Some(ref mut o) = out {
                                    let _ = o.start(Arc::clone(&audio_buffer));
                                }

                                {
                                    let mut s = state.lock().unwrap();
                                    s.is_playing = true;
                                    s.is_paused = false;
                                    s.duration_secs = Some(dec.duration_secs());
                                    s.current_position_secs = 0;
                                }

                                decoder = Some(dec);
                                resampler = Some(r);
                                output = out;
                                paused = false;
                            }
                            Err(e) => {
                                let _ = evt_tx.send(EngineEvent::Error(e.to_string()));
                            }
                        }
                    }
                    Err(e) => {
                        let _ = evt_tx.send(EngineEvent::Error(e.to_string()));
                    }
                }
            }
            Ok(Command::Pause) => {
                paused = true;
                let mut s = state.lock().unwrap();
                s.is_paused = true;
                s.is_playing = false;
            }
            Ok(Command::Resume) => {
                paused = false;
                let mut s = state.lock().unwrap();
                s.is_paused = false;
                s.is_playing = true;
            }
            Ok(Command::Stop) => {
                decoder = None;
                resampler = None;
                output = None;
                let mut s = state.lock().unwrap();
                s.is_playing = false;
                s.is_paused = false;
                s.current_position_secs = 0;
            }
            Ok(Command::Seek(pos)) => {
                if let Some(ref mut dec) = decoder {
                    let _ = dec.seek(pos);
                    let mut s = state.lock().unwrap();
                    s.current_position_secs = pos;
                }
            }
            Ok(Command::Shutdown) => {
                break;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
        }

        // Decode and output audio if playing
        if !paused {
            if let (Some(ref mut dec), Some(ref mut res)) = (&mut decoder, &mut resampler) {
                if !dec.is_finished() {
                    match dec.read_frame() {
                        Ok(Some((data, nb_samples))) => {
                            // Resample
                            let out_samples = nb_samples * 2; // rough estimate
                            let mut out_buf = vec![0f32; out_samples * 2];
                            let out_ptr = out_buf.as_mut_ptr() as *mut u8;
                            let in_ptr = data as *const u8;

                            match res.resample(
                                &in_ptr,
                                nb_samples as i32,
                                &mut (out_ptr as *mut u8),
                                out_samples as i32,
                            ) {
                                Ok(written) => {
                                    let samples = written as usize * 2;
                                    let mut buf = audio_buffer.lock().unwrap();
                                    buf.extend_from_slice(&out_buf[..samples]);

                                    // Update position
                                    let sample_rate = res.out_sample_rate() as u64;
                                    let channels = res.out_channels() as u64;
                                    if sample_rate > 0 && channels > 0 {
                                        let samples_played = written as u64;
                                        let mut s = state.lock().unwrap();
                                        s.current_position_secs +=
                                            samples_played / sample_rate;
                                    }
                                }
                                Err(e) => {
                                    let _ = evt_tx.send(EngineEvent::Error(e.to_string()));
                                }
                            }
                        }
                        Ok(None) => {
                            // Track finished
                            let _ = evt_tx.send(EngineEvent::TrackFinished);
                            decoder = None;
                            resampler = None;
                            let mut s = state.lock().unwrap();
                            s.is_playing = false;
                        }
                        Err(e) => {
                            let _ = evt_tx.send(EngineEvent::Error(e.to_string()));
                        }
                    }
                }
            }
        }

        // Small sleep to avoid busy waiting
        thread::sleep(std::time::Duration::from_millis(10));
    }
}

impl AudioEngine for FfmpegEngine {
    fn play(&mut self, path: &Path) -> anyhow::Result<()> {
        self.command_tx
            .send(Command::Play(path.to_string_lossy().to_string()))
            .map_err(|_| anyhow::anyhow!("Engine thread disconnected"))?;
        Ok(())
    }

    fn pause(&mut self) {
        let _ = self.command_tx.send(Command::Pause);
    }

    fn resume(&mut self) {
        let _ = self.command_tx.send(Command::Resume);
    }

    fn stop(&mut self) {
        let _ = self.command_tx.send(Command::Stop);
    }

    fn seek(&mut self, position_secs: u64) -> anyhow::Result<()> {
        self.command_tx
            .send(Command::Seek(position_secs))
            .map_err(|_| anyhow::anyhow!("Engine thread disconnected"))?;
        Ok(())
    }

    fn is_playing(&self) -> bool {
        self.state.lock().unwrap().is_playing
    }

    fn is_paused(&self) -> bool {
        self.state.lock().unwrap().is_paused
    }

    fn current_position_secs(&self) -> u64 {
        self.state.lock().unwrap().current_position_secs
    }

    fn duration_secs(&self) -> Option<u64> {
        self.state.lock().unwrap().duration_secs
    }

    fn poll_events(&mut self) -> Vec<EngineEvent> {
        let mut events = Vec::new();
        while let Ok(evt) = self.event_rx.try_recv() {
            events.push(evt);
        }
        events
    }
}

impl Drop for FfmpegEngine {
    fn drop(&mut self) {
        let _ = self.command_tx.send(Command::Shutdown);
    }
}
```

- [ ] **Step 3: Commit**

```bash
git add src/audio/ffmpeg/
git commit -m "feat: implement FfmpegEngine with decoder, resampler, and cpal output"
```

---

### Task 6: Update main.rs and config.rs imports

**Files:**
- Modify: `src/main.rs`
- Modify: `src/config.rs`
- Modify: `src/app.rs`

- [ ] **Step 1: Update `src/main.rs`**

Change:
```rust
use zotu::{
    app::Zotu, assets::Assets, config::Config, db::database::DB, error::log_error,
    play::player::Player,
};
```
To:
```rust
use zotu::{
    app::Zotu, assets::Assets, audio::ffmpeg::FfmpegEngine, audio::player::Player,
    config::Config, db::database::DB, error::log_error,
};
```

Change `Player::new()` to `Player::new(Box::new(FfmpegEngine::new().expect("Failed to create audio engine")))`.

- [ ] **Step 2: Update `src/config.rs`**

Change:
```rust
use crate::{db::metadata::AlbumInfo, play::player::LoopMode};
```
To:
```rust
use crate::{db::metadata::AlbumInfo, audio::playlist::LoopMode};
```

- [ ] **Step 3: Update `src/app.rs`**

Change:
```rust
use crate::{
    components::{...},
    db::{...},
    play::player::Player,
    theme::*,
    ui::search::{...},
};
```
To:
```rust
use crate::{
    components::{...},
    db::{...},
    audio::player::Player,
    theme::*,
    ui::search::{...},
};
```

- [ ] **Step 4: Commit**

```bash
git add src/main.rs src/config.rs src/app.rs
git commit -m "refactor: update imports for new audio module structure"
```

---

### Task 7: Update component imports

**Files:**
- Modify: `src/components/playbar.rs`
- Modify: `src/components/now_playing.rs`
- Modify: `src/components/songview.rs`

- [ ] **Step 1: Update playbar.rs imports**

Change:
```rust
use crate::{
    play::player::{LoopMode, PlayState, Player},
    theme::*,
    util::format_duration,
};
```
To:
```rust
use crate::{
    audio::player::Player,
    audio::playlist::{LoopMode, PlayState},
    theme::*,
    util::format_duration,
};
```

- [ ] **Step 2: Update now_playing.rs imports**

Change:
```rust
use crate::{
    db::metadata::AlbumInfo,
    play::player::{LoopMode, PlayState, Player},
    theme::*,
    util::format_duration,
};
```
To:
```rust
use crate::{
    db::metadata::AlbumInfo,
    audio::player::Player,
    audio::playlist::{LoopMode, PlayState},
    theme::*,
    util::format_duration,
};
```

- [ ] **Step 3: Update songview.rs imports**

Change:
```rust
use crate::{
    db::{...},
    play::player::Player,
    theme::*,
    ui::menu::{...},
    util::format_duration,
};
```
To:
```rust
use crate::{
    db::{...},
    audio::player::Player,
    theme::*,
    ui::menu::{...},
    util::format_duration,
};
```

- [ ] **Step 4: Commit**

```bash
git add src/components/
git commit -m "refactor: update component imports for new audio module"
```

---

### Task 8: Update Cargo.toml and remove old files

**Files:**
- Modify: `Cargo.toml`
- Delete: `src/play.rs`
- Delete: `src/play/player.rs`
- Delete: `src/ffmpeg/` (entire directory)

- [ ] **Step 1: Update Cargo.toml**

Remove `rodio` from dependencies, add `cpal`:

```toml
[dependencies]
crossbeam = "0.8.4"
gpui = "0.2.2"
image = "0.25.9"
rand = "0.9"
rfd = "0.17.2"
rusqlite = { version = "0.38.0", features = ["bundled"] }
serde = "1.0.228"
serde_json = "1.0.149"
tracing = "0.1.44"
tracing-subscriber = "0.3.22"
uuid = { version = "1.19.0", features = ["v4"] }
walkdir = "2.5.0"
cpal = "0.15"
anyhow = "1.0"
```

(rodio is removed, cpal stays)

- [ ] **Step 2: Delete old files**

```bash
rm -rf src/play.rs src/play/ src/ffmpeg/
```

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git rm -r src/play.rs src/play/ src/ffmpeg/
git commit -m "refactor: remove rodio and old module structure"
```

---

### Task 9: Verify compilation and fix issues

- [ ] **Step 1: Run cargo check**

```bash
cargo check 2>&1
```

Expected: Compilation errors due to incomplete FFI integration (stream finding, codec detection). Fix iteratively.

- [ ] **Step 2: Fix FFI integration issues**

The main issues will be:
1. Finding the correct audio stream in `FfmpegDecoder::open`
2. Getting codec parameters from stream
3. Proper pointer management

Add helper functions to `ffi.rs` as needed.

- [ ] **Step 3: Run cargo check again**

```bash
cargo check 2>&1
```

Expected: Clean compilation.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "fix: resolve compilation issues after audio backend refactor"
```

---

### Task 10: Final cleanup and verification

- [ ] **Step 1: Run cargo build**

```bash
cargo build 2>&1
```

Expected: Successful build.

- [ ] **Step 2: Verify no unused imports**

```bash
cargo check 2>&1 | grep "unused import"
```

Expected: No warnings.

- [ ] **Step 3: Final commit**

```bash
git add -A
git commit -m "refactor: complete audio backend migration from rodio to ffmpeg+cpal"
```

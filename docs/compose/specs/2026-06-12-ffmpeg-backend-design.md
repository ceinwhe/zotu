# Zotu FFmpeg Audio Backend Refactoring Design

## [S1] Problem

Zotu currently uses Rodio for audio playback, which provides limited functionality and lacks precise seeking capabilities. The user wants to replace the audio backend with FFmpeg for:
- Complete audio format support
- Precise seeking capabilities
- Advanced audio processing features
- Full control over unsafe FFI code through safe Rust abstractions

## [S2] Solution Overview

Replace Rodio with a custom FFmpeg backend implemented through:
1. Hand-written FFI bindings (not using existing crates)
2. Safe Rust abstractions using RAII patterns
3. Modular architecture for extensibility
4. Application-layer playback management

## [S3] Architecture

### Layer 1: Hand-written FFI Bindings

Create Rust bindings for FFmpeg core functions:

**Type Bindings:**
- `AVCodecContext` → `FfmpegCodecContext`
- `AVFrame` → `FfmpegFrame`
- `AVPacket` → `FfmpegPacket`
- `AVFormatContext` → `FfmpegFormatContext`
- `SwrContext` → `FfmpegSwrContext`
- `AVFilterGraph` → `FfmpegFilterGraph`

**Function Bindings:**
- `avcodec_open2`, `avcodec_send_packet`, `avcodec_receive_frame`
- `avformat_open_input`, `avformat_find_stream_info`
- `swr_alloc_set_opts`, `swr_convert`
- `avfilter_graph_parse_ptr`, `avfilter_graph_config`

**Implementation Details:**
- Use `#[repr(C)]` for memory layout compatibility
- Use `extern "C"` for function declarations
- Manual memory management with Drop trait implementations

### Layer 2: Safe Abstractions

**Module: `ffmpeg_decoder`**
```rust
pub struct Decoder {
    codec_context: FfmpegCodecContext,
    // ...
}

impl Decoder {
    pub fn new(path: &Path) -> anyhow::Result<Self>;
    pub fn decode(&mut self) -> anyhow::Result<Option<AudioFrame>>;
    pub fn flush(&mut self) -> anyhow::Result<()>;
    pub fn seek(&mut self, timestamp: f64) -> anyhow::Result<()>;
}

impl Drop for Decoder { /* cleanup */ }
```

**Module: `ffmpeg_metadata`**
```rust
pub struct MetadataReader {
    format_context: FfmpegFormatContext,
}

impl MetadataReader {
    pub fn new(path: &Path) -> anyhow::Result<Self>;
    pub fn read_metadata(&self) -> anyhow::Result<AudioMetadata>;
    pub fn get_cover_art(&self) -> anyhow::Result<Option<Vec<u8>>>;
}

pub struct AudioMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: f64,
    pub tags: HashMap<String, String>,
}
```

**Module: `ffmpeg_resampler`**
```rust
pub struct Resampler {
    swr_context: FfmpegSwrContext,
}

impl Resampler {
    pub fn new(
        in_sample_rate: i32,
        in_channels: i32,
        in_format: SampleFormat,
        out_sample_rate: i32,
        out_channels: i32,
        out_format: SampleFormat,
    ) -> anyhow::Result<Self>;
    
    pub fn resample(&mut self, input: &AudioFrame) -> anyhow::Result<AudioFrame>;
}

impl Drop for Resampler { /* cleanup */ }
```

**Module: `ffmpeg_filters`**
```rust
pub struct FilterGraph {
    graph: FfmpegFilterGraph,
}

impl FilterGraph {
    pub fn new() -> anyhow::Result<Self>;
    pub fn add_equalizer(&mut self, freq: f64, gain: f64) -> anyhow::Result<()>;
    pub fn add_reverb(&mut self, room_size: f64) -> anyhow::Result<()>;
    pub fn apply(&mut self, input: &AudioFrame) -> anyhow::Result<AudioFrame>;
}

impl Drop for FilterGraph { /* cleanup */ }
```

**Module: `ffmpeg_output`**
```rust
pub trait AudioOutput {
    fn play(&mut self, frame: &AudioFrame) -> anyhow::Result<()>;
    fn pause(&mut self) -> anyhow::Result<()>;
    fn resume(&mut self) -> anyhow::Result<()>;
    fn stop(&mut self) -> anyhow::Result<()>;
    fn set_volume(&mut self, volume: f32) -> anyhow::Result<()>;
}

pub struct CpalOutput {
    // CPAL stream handle
}

impl AudioOutput for CpalOutput { /* implementation */ }
```

### Layer 3: Application-layer Playback Management

**Module: `playlist`**
```rust
pub struct Playlist {
    songs: Vec<SongInfo>,
    current_index: usize,
}

impl Playlist {
    pub fn new() -> Self;
    pub fn add(&mut self, song: SongInfo);
    pub fn remove(&mut self, index: usize);
    pub fn shuffle(&mut self);
    pub fn clear(&mut self);
    pub fn current(&self) -> Option<&SongInfo>;
    pub fn next(&mut self) -> Option<&SongInfo>;
    pub fn previous(&mut self) -> Option<&SongInfo>;
}
```

**Module: `play_mode`**
```rust
pub enum PlayMode {
    Sequential,
    Loop,
    Single,
    Random,
}

impl PlayMode {
    pub fn next_index(&self, current: usize, length: usize) -> Option<usize>;
}
```

**Module: `play_history`**
```rust
pub struct PlayHistory {
    history: Vec<SongInfo>,
    position: usize,
}

impl PlayHistory {
    pub fn new() -> Self;
    pub fn push(&mut self, song: SongInfo);
    pub fn back(&mut self) -> Option<&SongInfo>;
    pub fn forward(&mut self) -> Option<&SongInfo>;
}
```

**Module: `player_controller`**
```rust
pub struct PlayerController {
    decoder: Option<Decoder>,
    resampler: Option<Resampler>,
    output: Box<dyn AudioOutput>,
    playlist: Playlist,
    play_mode: PlayMode,
    history: PlayHistory,
    is_playing: bool,
    volume: f32,
}

impl PlayerController {
    pub fn new(output: Box<dyn AudioOutput>) -> Self;
    pub fn play(&mut self, song: &SongInfo) -> anyhow::Result<()>;
    pub fn pause(&mut self) -> anyhow::Result<()>;
    pub fn resume(&mut self) -> anyhow::Result<()>;
    pub fn stop(&mut self) -> anyhow::Result<()>;
    pub fn next(&mut self) -> anyhow::Result<()>;
    pub fn previous(&mut self) -> anyhow::Result<()>;
    pub fn seek(&mut self, position: f64) -> anyhow::Result<()>;
    pub fn set_volume(&mut self, volume: f32) -> anyhow::Result<()>;
    pub fn set_play_mode(&mut self, mode: PlayMode);
    pub fn progress(&self) -> f64;
    pub fn duration(&self) -> f64;
}
```

## [S4] Integration with Existing Code

**Changes to `src/play/player.rs`:**
- Remove Rodio dependencies
- Replace `OutputStream` and `Sink` with `PlayerController`
- Keep existing `Player` struct interface (Global trait)
- Update all methods to use new controller

**Changes to `src/db/metadata.rs`:**
- Remove `lofty` dependency
- Replace with `ffmpeg_metadata` module
- Keep `AlbumInfo` struct interface

**Changes to `Cargo.toml`:**
- Remove `rodio` dependency
- Add `cpal` for audio output
- Add `anyhow` for error handling
- Add build script for FFmpeg linking

## [S5] Build Configuration

**FFmpeg Static Linking:**
- Create `build.rs` script for FFmpeg compilation
- Use `pkg-config` or `vcpkg` for FFmpeg discovery
- Support Windows, macOS, Linux

**Dependencies:**
```toml
[dependencies]
cpal = "0.15"
anyhow = "1.0"
# Remove rodio

[build-dependencies]
pkg-config = "0.3"
```

## [S6] Testing Strategy

**Unit Tests:**
- Test each FFmpeg wrapper module independently
- Test error handling paths
- Test resource cleanup (Drop implementations)

**Integration Tests:**
- Test complete audio pipeline
- Test playlist management
- Test playback modes

**Performance Tests:**
- Decode performance benchmarks
- Memory usage measurements
- Latency measurements

**Compatibility Tests:**
- Test multiple audio formats (MP3, FLAC, OGG, WAV, AAC)
- Test different sample rates and channel configurations
- Test cross-platform compatibility

## [S7] Implementation Phases

**Phase 1: FFI Bindings**
- Implement core FFmpeg type bindings
- Implement essential function bindings
- Add basic error handling

**Phase 2: Safe Abstractions**
- Implement Decoder module
- Implement MetadataReader module
- Implement Resampler module

**Phase 3: Audio Output**
- Implement AudioOutput trait
- Implement CpalOutput
- Test basic playback

**Phase 4: Application Layer**
- Implement Playlist management
- Implement PlayMode
- Implement PlayHistory
- Implement PlayerController

**Phase 5: Integration**
- Update existing Player struct
- Update metadata reading
- Update UI components

**Phase 6: Testing**
- Write unit tests
- Write integration tests
- Performance optimization

## [S8] Risk Assessment

**Technical Risks:**
- FFmpeg API complexity
- Memory safety with manual FFI
- Cross-platform build configuration
- Audio synchronization

**Mitigations:**
- Comprehensive testing
- RAII patterns for resource management
- CI/CD for cross-platform builds
- Use proven audio output library (CPAL)

## [S9] Success Criteria

1. All existing audio formats supported
2. Precise seeking works correctly
3. No memory leaks
4. Performance comparable to or better than Rodio
5. All existing UI features work unchanged
6. Code compiles without unsafe in public API

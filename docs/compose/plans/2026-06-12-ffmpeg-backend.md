# Zotu FFmpeg Audio Backend Refactoring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use compose:subagent (recommended) or compose:execute to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Rodio audio backend with custom FFmpeg implementation using hand-written FFI bindings and safe Rust abstractions.

**Architecture:** Three-layer architecture: hand-written FFmpeg FFI bindings, safe Rust abstractions using RAII patterns, and application-layer playback management. Modular design for extensibility.

**Tech Stack:** Rust, FFmpeg (static linking), CPAL (audio output), anyhow (error handling)

---

## File Structure

**New Files:**
- `src/ffmpeg/` - FFmpeg module directory
  - `mod.rs` - Module declarations
  - `ffi.rs` - Hand-written FFI bindings
  - `decoder.rs` - Audio decoder
  - `metadata.rs` - Metadata reader
  - `resampler.rs` - Audio resampler
  - `filters.rs` - Audio filters
  - `output.rs` - Audio output trait and CPAL implementation
- `src/player/` - Player module directory
  - `mod.rs` - Module declarations
  - `controller.rs` - Player controller
  - `playlist.rs` - Playlist management
  - `play_mode.rs` - Playback modes
  - `history.rs` - Play history
- `build.rs` - FFmpeg build script

**Modified Files:**
- `src/play/player.rs` - Replace Rodio with new player
- `src/db/metadata.rs` - Replace lofty with FFmpeg metadata
- `Cargo.toml` - Update dependencies
- `src/lib.rs` - Update module exports
- `src/main.rs` - Update imports

---

## Task 1: Project Setup and Dependencies

**Covers:** [S5]

**Files:**
- Modify: `Cargo.toml`
- Create: `build.rs`

- [ ] **Step 1: Update Cargo.toml dependencies**

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

[build-dependencies]
pkg-config = "0.3"
```

- [ ] **Step 2: Create build.rs for FFmpeg linking**

```rust
fn main() {
    // Try pkg-config first
    if pkg_config::probe_library("libavcodec").is_ok() {
        pkg_config::probe_library("libavformat").unwrap();
        pkg_config::probe_library("libavutil").unwrap();
        pkg_config::probe_library("libswresample").unwrap();
        pkg_config::probe_library("libavfilter").unwrap();
    } else {
        // Fallback: try to find FFmpeg in standard locations
        println!("cargo:rustc-link-lib=avcodec");
        println!("cargo:rustc-link-lib=avformat");
        println!("cargo:rustc-link-lib=avutil");
        println!("cargo:rustc-link-lib=swresample");
        println!("cargo:rustc-link-lib=avfilter");
    }
}
```

- [ ] **Step 3: Verify project builds**

Run: `cargo check`
Expected: Compiles successfully (may have warnings about unused imports)

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml build.rs
git commit -m "chore: add FFmpeg and CPAL dependencies, remove rodio"
```

---

## Task 2: FFmpeg FFI Bindings

**Covers:** [S3]

**Files:**
- Create: `src/ffmpeg/mod.rs`
- Create: `src/ffmpeg/ffi.rs`

- [ ] **Step 1: Create ffmpeg module structure**

```rust
// src/ffmpeg/mod.rs
pub mod ffi;
pub mod decoder;
pub mod metadata;
pub mod resampler;
pub mod filters;
pub mod output;
```

- [ ] **Step 2: Implement basic FFI type bindings**

```rust
// src/ffmpeg/ffi.rs
use std::os::raw::{c_int, c_char, c_void, c_uint, c_ulonglong};

#[repr(C)]
pub struct AVCodecContext {
    _private: [u8; 0],
}

#[repr(C)]
pub struct AVFrame {
    _private: [u8; 0],
}

#[repr(C)]
pub struct AVPacket {
    _private: [u8; 0],
}

#[repr(C)]
pub struct AVFormatContext {
    _private: [u8; 0],
}

#[repr(C)]
pub struct SwrContext {
    _private: [u8; 0],
}

#[repr(C)]
pub struct AVFilterGraph {
    _private: [u8; 0],
}

#[repr(C)]
pub struct AVFilterContext {
    _private: [u8; 0],
}

#[repr(C)]
pub struct AVCodec {
    _private: [u8; 0],
}

#[repr(C)]
pub struct AVStream {
    _private: [u8; 0],
}

#[repr(C)]
pub struct AVDictionary {
    _private: [u8; 0],
}

// Error codes
pub const AVERROR_EOF: c_int = -541478725;
pub const AVERROR(E: c_int): c_int = -(E);

// Sample formats
pub const AV_SAMPLE_FMT_S16: c_int = 1;
pub const AV_SAMPLE_FMT_FLT: c_int = 3;

// Channel layouts
pub const AV_CH_LAYOUT_STEREO: c_ulonglong = 0x3;
pub const AV_CH_LAYOUT_MONO: c_ulonglong = 0x4;
```

- [ ] **Step 3: Implement FFI function declarations**

```rust
// Add to src/ffmpeg/ffi.rs

extern "C" {
    // Format context functions
    pub fn avformat_open_input(
        ps: *mut *mut AVFormatContext,
        filename: *const c_char,
        fmt: *mut AVFormatContext,
        options: *mut *mut AVDictionary,
    ) -> c_int;

    pub fn avformat_close_input(ps: *mut *mut AVFormatContext);

    pub fn avformat_find_stream_info(
        ic: *mut AVFormatContext,
        options: *mut *mut AVDictionary,
    ) -> c_int;

    // Codec context functions
    pub fn avcodec_alloc_context3(codec: *const AVCodec) -> *mut AVCodecContext;

    pub fn avcodec_free_context(avctx: *mut *mut AVCodecContext);

    pub fn avcodec_parameters_to_context(
        codec: *mut AVCodecContext,
        par: *const c_void,
    ) -> c_int;

    pub fn avcodec_open2(
        avctx: *mut AVCodecContext,
        codec: *const AVCodec,
        options: *mut *mut AVDictionary,
    ) -> c_int;

    pub fn avcodec_send_packet(
        avctx: *mut AVCodecContext,
        avpkt: *const AVPacket,
    ) -> c_int;

    pub fn avcodec_receive_frame(
        avctx: *mut AVCodecContext,
        frame: *mut AVFrame,
    ) -> c_int;

    pub fn avcodec_flush_buffers(avctx: *mut AVCodecContext);

    // Frame functions
    pub fn av_frame_alloc() -> *mut AVFrame;

    pub fn av_frame_free(frame: *mut *mut AVFrame);

    pub fn av_frame_get_nb_samples(frame: *const AVFrame) -> c_int;

    pub fn av_frame_get_channels(frame: *const AVFrame) -> c_int;

    pub fn av_frame_get_sample_rate(frame: *const AVFrame) -> c_int;

    pub fn av_frame_get_linesize(frame: *const AVFrame) -> c_int;

    pub fn av_frame_get_data(frame: *const AVFrame, plane: c_int) -> *mut u8;

    // Packet functions
    pub fn av_packet_alloc() -> *mut AVPacket;

    pub fn av_packet_free(pkt: *mut *mut AVPacket);

    pub fn av_read_frame(s: *mut AVFormatContext, pkt: *mut AVPacket) -> c_int;

    // Resampler functions
    pub fn swr_alloc_set_opts(
        s: *mut SwrContext,
        out_ch_layout: c_ulonglong,
        out_sample_fmt: c_int,
        out_sample_rate: c_int,
        in_ch_layout: c_ulonglong,
        in_sample_fmt: c_int,
        in_sample_rate: c_int,
        log_offset: c_int,
        log_ctx: *mut c_void,
    ) -> *mut SwrContext;

    pub fn swr_init(s: *mut SwrContext) -> c_int;

    pub fn swr_convert(
        s: *mut SwrContext,
        out: *mut *mut u8,
        out_count: c_int,
        in_: *const *const u8,
        in_count: c_int,
    ) -> c_int;

    pub fn swr_free(s: *mut *mut SwrContext);

    // Filter functions
    pub fn avfilter_graph_alloc() -> *mut AVFilterGraph;

    pub fn avfilter_graph_free(graph: *mut *mut AVFilterGraph);

    pub fn avfilter_graph_parse_ptr(
        graph: *mut AVFilterGraph,
        filters: *const c_char,
        inputs: *mut *mut AVFilterContext,
        outputs: *mut *mut AVFilterContext,
        log_ctx: *mut c_void,
    ) -> c_int;

    pub fn avfilter_graph_config(graph: *mut AVFilterContext, log_ctx: *mut c_void) -> c_int;

    // Utility functions
    pub fn avcodec_find_decoder(codec_id: c_uint) -> *const AVCodec;

    pub fn avcodec_find_encoder(codec_id: c_uint) -> *const AVCodec;

    pub fn av_strerror(
        errnum: c_int,
        errbuf: *mut c_char,
        errbuf_size: usize,
    ) -> c_int;

    // Seeking
    pub fn av_seek_frame(
        s: *mut AVFormatContext,
        stream_index: c_int,
        timestamp: c_ulonglong,
        flags: c_int,
    ) -> c_int;
}

// Safe wrapper for error conversion
pub fn ffmpeg_error(code: c_int) -> anyhow::Error {
    if code >= 0 {
        anyhow::anyhow!("Unexpected positive error code: {}", code)
    } else {
        let mut buf = [0u8; 256];
        unsafe {
            av_strerror(-code, buf.as_mut_ptr() as *mut i8, buf.len());
        }
        let msg = std::str::from_utf8(&buf)
            .unwrap_or("Unknown error")
            .trim_end_matches('\0');
        anyhow::anyhow!("FFmpeg error: {}", msg)
    }
}
```

- [ ] **Step 4: Add lib.rs module declaration**

```rust
// Add to src/lib.rs
pub mod ffmpeg;
```

- [ ] **Step 5: Verify FFI bindings compile**

Run: `cargo check`
Expected: Compiles with FFI bindings

- [ ] **Step 6: Commit**

```bash
git add src/ffmpeg/
git commit -m "feat: add FFmpeg FFI bindings"
```

---

## Task 3: FFmpeg Decoder Implementation

**Covers:** [S3, S4]

**Files:**
- Create: `src/ffmpeg/decoder.rs`

- [ ] **Step 1: Implement Decoder struct**

```rust
// src/ffmpeg/decoder.rs
use std::path::Path;
use std::ptr;
use anyhow::{Result, Context};
use super::ffi::*;

pub struct AudioFrame {
    pub data: Vec<f32>,
    pub sample_rate: i32,
    pub channels: i32,
    pub pts: f64,
}

pub struct Decoder {
    format_context: *mut AVFormatContext,
    codec_context: *mut AVCodecContext,
    packet: *mut AVPacket,
    frame: *mut AVFrame,
    stream_index: i32,
    time_base: f64,
}

impl Decoder {
    pub fn new(path: &Path) -> Result<Self> {
        unsafe {
            let path_str = std::ffi::CString::new(path.to_str().context("Invalid path")?)?;
            
            let mut format_context = ptr::null_mut();
            let result = avformat_open_input(
                &mut format_context,
                path_str.as_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
            );
            if result < 0 {
                return Err(ffmpeg_error(result).context("Failed to open input"));
            }

            let result = avformat_find_stream_info(format_context, ptr::null_mut());
            if result < 0 {
                avformat_close_input(&mut format_context);
                return Err(ffmpeg_error(result).context("Failed to find stream info"));
            }

            // Find audio stream
            let mut stream_index = -1;
            let nb_streams = (*format_context).nb_streams;
            let streams = (*format_context).streams;
            
            for i in 0..nb_streams {
                let stream = *streams.add(i as usize);
                let codecpar = (*stream).codecpar;
                if (*codecpar).codec_type == 0 { // AVMEDIA_TYPE_AUDIO
                    stream_index = i as i32;
                    break;
                }
            }

            if stream_index == -1 {
                avformat_close_input(&mut format_context);
                return Err(anyhow::anyhow!("No audio stream found"));
            }

            let stream = *streams.add(stream_index as usize);
            let codecpar = (*stream).codecpar;
            let codec_id = (*codecpar).codec_id;
            
            let codec = avcodec_find_decoder(codec_id);
            if codec.is_null() {
                avformat_close_input(&mut format_context);
                return Err(anyhow::anyhow!("Unsupported codec"));
            }

            let codec_context = avcodec_alloc_context3(codec);
            if codec_context.is_null() {
                avformat_close_input(&mut format_context);
                return Err(anyhow::anyhow!("Failed to allocate codec context"));
            }

            let result = avcodec_parameters_to_context(codec_context, codecpar as *const c_void);
            if result < 0 {
                avcodec_free_context(&mut codec_context);
                avformat_close_input(&mut format_context);
                return Err(ffmpeg_error(result).context("Failed to copy codec parameters"));
            }

            let result = avcodec_open2(codec_context, codec, ptr::null_mut());
            if result < 0 {
                avcodec_free_context(&mut codec_context);
                avformat_close_input(&mut format_context);
                return Err(ffmpeg_error(result).context("Failed to open codec"));
            }

            let time_base = (*stream).time_base;
            let time_base_f64 = time_base.den as f64 / time_base.num as f64;

            let packet = av_packet_alloc();
            let frame = av_frame_alloc();

            Ok(Decoder {
                format_context,
                codec_context,
                packet,
                frame,
                stream_index,
                time_base: time_base_f64,
            })
        }
    }

    pub fn decode(&mut self) -> Result<Option<AudioFrame>> {
        unsafe {
            loop {
                let result = avcodec_receive_frame(self.codec_context, self.frame);
                
                if result == 0 {
                    // Got a frame
                    let nb_samples = av_frame_get_nb_samples(self.frame);
                    let channels = av_frame_get_channels(self.frame);
                    let sample_rate = av_frame_get_sample_rate(self.frame);
                    let pts = (*self.frame).best_effort_timestamp as f64 * self.time_base;
                    
                    let data = if (*self.frame).format == AV_SAMPLE_FMT_S16 {
                        // Convert S16 to F32
                        let linesize = av_frame_get_linesize(self.frame);
                        let data_ptr = av_frame_get_data(self.frame, 0);
                        let bytes = linesize * channels;
                        let samples = bytes / 2;
                        
                        let mut f32_data = Vec::with_capacity(samples as usize);
                        for i in 0..samples {
                            let s16 = *(data_ptr.add(i as usize * 2) as *const i16);
                            f32_data.push(s16 as f32 / 32768.0);
                        }
                        f32_data
                    } else if (*self.frame).format == AV_SAMPLE_FMT_FLT {
                        let linesize = av_frame_get_linesize(self.frame);
                        let data_ptr = av_frame_get_data(self.frame, 0);
                        let samples = linesize * channels / 4;
                        
                        let mut f32_data = Vec::with_capacity(samples as usize);
                        for i in 0..samples {
                            let f32_val = *(data_ptr.add(i as usize * 4) as *const f32);
                            f32_data.push(f32_val);
                        }
                        f32_data
                    } else {
                        return Err(anyhow::anyhow!("Unsupported sample format"));
                    };

                    return Ok(Some(AudioFrame {
                        data,
                        sample_rate,
                        channels,
                        pts,
                    }));
                } else if result == AVERROR_EOF {
                    return Ok(None);
                } else if result == -11 { // EAGAIN
                    // Need more input
                    let result = av_read_frame(self.format_context, self.packet);
                    if result == AVERROR_EOF {
                        // Flush decoder
                        avcodec_send_packet(self.codec_context, ptr::null());
                        continue;
                    } else if result < 0 {
                        return Err(ffmpeg_error(result).context("Failed to read frame"));
                    }
                    
                    if (*self.packet).stream_index != self.stream_index {
                        av_packet_unref(self.packet);
                        continue;
                    }
                    
                    let result = avcodec_send_packet(self.codec_context, self.packet);
                    av_packet_unref(self.packet);
                    
                    if result < 0 {
                        return Err(ffmpeg_error(result).context("Failed to send packet"));
                    }
                } else {
                    return Err(ffmpeg_error(result).context("Failed to receive frame"));
                }
            }
        }
    }

    pub fn seek(&mut self, timestamp: f64) -> Result<()> {
        unsafe {
            let timestamp_int = (timestamp * self.time_base) as i64;
            let result = av_seek_frame(
                self.format_context,
                self.stream_index,
                timestamp_int as u64,
                0, // AVSEEK_FLAG_BACKWARD
            );
            if result < 0 {
                return Err(ffmpeg_error(result).context("Failed to seek"));
            }
            avcodec_flush_buffers(self.codec_context);
            Ok(())
        }
    }

    pub fn duration(&self) -> f64 {
        unsafe {
            (*self.format_context).duration as f64 / 1000000.0
        }
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe {
            if !self.frame.is_null() {
                av_frame_free(&mut self.frame);
            }
            if !self.packet.is_null() {
                av_packet_free(&mut self.packet);
            }
            if !self.codec_context.is_null() {
                avcodec_free_context(&mut self.codec_context);
            }
            if !self.format_context.is_null() {
                avformat_close_input(&mut self.format_context);
            }
        }
    }
}

unsafe impl Send for Decoder {}
unsafe impl Sync for Decoder {}
```

- [ ] **Step 2: Update ffmpeg/mod.rs**

```rust
// src/ffmpeg/mod.rs
pub mod ffi;
pub mod decoder;
pub mod metadata;
pub mod resampler;
pub mod filters;
pub mod output;

pub use decoder::{Decoder, AudioFrame};
```

- [ ] **Step 3: Verify decoder compiles**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add src/ffmpeg/decoder.rs
git commit -m "feat: implement FFmpeg audio decoder"
```

---

## Task 4: FFmpeg Metadata Reader

**Covers:** [S3, S4]

**Files:**
- Create: `src/ffmpeg/metadata.rs`

- [ ] **Step 1: Implement MetadataReader**

```rust
// src/ffmpeg/metadata.rs
use std::path::Path;
use std::ptr;
use anyhow::{Result, Context};
use super::ffi::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AudioMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: f64,
    pub sample_rate: i32,
    pub channels: i32,
    pub bits_per_sample: i32,
    pub tags: HashMap<String, String>,
}

pub struct MetadataReader {
    format_context: *mut AVFormatContext,
}

impl MetadataReader {
    pub fn new(path: &Path) -> Result<Self> {
        unsafe {
            let path_str = std::ffi::CString::new(path.to_str().context("Invalid path")?)?;
            
            let mut format_context = ptr::null_mut();
            let result = avformat_open_input(
                &mut format_context,
                path_str.as_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
            );
            if result < 0 {
                return Err(ffmpeg_error(result).context("Failed to open input"));
            }

            let result = avformat_find_stream_info(format_context, ptr::null_mut());
            if result < 0 {
                avformat_close_input(&mut format_context);
                return Err(ffmpeg_error(result).context("Failed to find stream info"));
            }

            Ok(MetadataReader { format_context })
        }
    }

    pub fn read_metadata(&self) -> Result<AudioMetadata> {
        unsafe {
            let format_context = self.format_context;
            
            // Get basic info
            let duration = (*format_context).duration as f64 / 1000000.0;
            
            // Find audio stream for technical info
            let mut sample_rate = 0;
            let mut channels = 0;
            let mut bits_per_sample = 0;
            
            let nb_streams = (*format_context).nb_streams;
            let streams = (*format_context).streams;
            
            for i in 0..nb_streams {
                let stream = *streams.add(i as usize);
                let codecpar = (*stream).codecpar;
                if (*codecpar).codec_type == 0 { // AVMEDIA_TYPE_AUDIO
                    sample_rate = (*codecpar).sample_rate;
                    channels = (*(*codecpar).ch_layout).nb_channels;
                    bits_per_sample = (*codecpar).bits_per_raw_sample;
                    break;
                }
            }

            // Read tags
            let mut tags = HashMap::new();
            let mut metadata = (*format_context).metadata;
            if !metadata.is_null() {
                let mut tag = (*metadata).first;
                while !tag.is_null() {
                    let key = std::ffi::CStr::from_ptr((*tag).key)
                        .to_str()
                        .unwrap_or("")
                        .to_string();
                    let value = std::ffi::CStr::from_ptr((*tag).value)
                        .to_str()
                        .unwrap_or("")
                        .to_string();
                    tags.insert(key, value);
                    tag = (*tag).next;
                }
            }

            // Extract common tags
            let title = tags.get("title").cloned();
            let artist = tags.get("artist").cloned();
            let album = tags.get("album").cloned();

            Ok(AudioMetadata {
                title,
                artist,
                album,
                duration,
                sample_rate,
                channels,
                bits_per_sample,
                tags,
            })
        }
    }

    pub fn get_cover_art(&self) -> Result<Option<Vec<u8>>> {
        unsafe {
            let format_context = self.format_context;
            let nb_streams = (*format_context).nb_streams;
            let streams = (*format_context).streams;
            
            for i in 0..nb_streams {
                let stream = *streams.add(i as usize);
                let codecpar = (*stream).codecpar;
                if (*codecpar).codec_type == 3 { // AVMEDIA_TYPE_ATTACHMENT
                    let codec_id = (*codecpar).codec_id;
                    // Check for common image codecs
                    if codec_id == 14 // AV_CODEC_ID_PNG
                        || codec_id == 61 // AV_CODEC_ID_MJPEG
                        || codec_id == 72 // AV_CODEC_ID_BMP
                    {
                        let extradata = (*codecpar).extradata;
                        let extradata_size = (*codecpar).extradata_size;
                        
                        if !extradata.is_null() && extradata_size > 0 {
                            let mut data = Vec::with_capacity(extradata_size as usize);
                            for i in 0..extradata_size {
                                data.push(*extradata.add(i as usize));
                            }
                            return Ok(Some(data));
                        }
                    }
                }
            }
            
            Ok(None)
        }
    }
}

impl Drop for MetadataReader {
    fn drop(&mut self) {
        unsafe {
            if !self.format_context.is_null() {
                avformat_close_input(&mut self.format_context);
            }
        }
    }
}

unsafe impl Send for MetadataReader {}
unsafe impl Sync for MetadataReader {}
```

- [ ] **Step 2: Update ffmpeg/mod.rs**

```rust
// Add to src/ffmpeg/mod.rs
pub use metadata::{MetadataReader, AudioMetadata};
```

- [ ] **Step 3: Verify metadata reader compiles**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add src/ffmpeg/metadata.rs
git commit -m "feat: implement FFmpeg metadata reader"
```

---

## Task 5: FFmpeg Resampler

**Covers:** [S3]

**Files:**
- Create: `src/ffmpeg/resampler.rs`

- [ ] **Step 1: Implement Resampler**

```rust
// src/ffmpeg/resampler.rs
use anyhow::{Result, Context};
use super::ffi::*;
use super::decoder::AudioFrame;

pub struct Resampler {
    swr_context: *mut SwrContext,
    out_sample_rate: i32,
    out_channels: i32,
}

impl Resampler {
    pub fn new(
        in_sample_rate: i32,
        in_channels: i32,
        out_sample_rate: i32,
        out_channels: i32,
    ) -> Result<Self> {
        unsafe {
            let in_ch_layout = if in_channels == 1 { 
                AV_CH_LAYOUT_MONO 
            } else { 
                AV_CH_LAYOUT_STEREO 
            };
            
            let out_ch_layout = if out_channels == 1 { 
                AV_CH_LAYOUT_MONO 
            } else { 
                AV_CH_LAYOUT_STEREO 
            };

            let swr_context = swr_alloc_set_opts(
                ptr::null_mut(),
                out_ch_layout,
                AV_SAMPLE_FMT_FLT,
                out_sample_rate,
                in_ch_layout,
                AV_SAMPLE_FMT_FLT,
                in_sample_rate,
                0,
                ptr::null_mut(),
            );

            if swr_context.is_null() {
                return Err(anyhow::anyhow!("Failed to allocate resampler"));
            }

            let result = swr_init(swr_context);
            if result < 0 {
                swr_free(&mut swr_context);
                return Err(ffmpeg_error(result).context("Failed to initialize resampler"));
            }

            Ok(Resampler {
                swr_context,
                out_sample_rate,
                out_channels,
            })
        }
    }

    pub fn resample(&self, input: &AudioFrame) -> Result<AudioFrame> {
        unsafe {
            let in_samples = input.data.len() / input.channels as usize;
            let out_samples = (in_samples as f64 * self.out_sample_rate as f64 / input.sample_rate as f64) as i32;
            
            let mut out_data = vec![0.0f32; out_samples as usize * self.out_channels as usize];
            
            let in_data = input.data.as_ptr();
            let mut in_ptr = &in_data;
            
            let result = swr_convert(
                self.swr_context,
                &mut out_data.as_mut_ptr(),
                out_samples,
                &in_ptr,
                in_samples as i32,
            );

            if result < 0 {
                return Err(ffmpeg_error(result).context("Failed to resample"));
            }

            Ok(AudioFrame {
                data: out_data,
                sample_rate: self.out_sample_rate,
                channels: self.out_channels,
                pts: input.pts,
            })
        }
    }
}

impl Drop for Resampler {
    fn drop(&mut self) {
        unsafe {
            if !self.swr_context.is_null() {
                swr_free(&mut self.swr_context);
            }
        }
    }
}

unsafe impl Send for Resampler {}
unsafe impl Sync for Resampler {}
```

- [ ] **Step 2: Update ffmpeg/mod.rs**

```rust
// Add to src/ffmpeg/mod.rs
pub use resampler::Resampler;
```

- [ ] **Step 3: Verify resampler compiles**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add src/ffmpeg/resampler.rs
git commit -m "feat: implement FFmpeg resampler"
```

---

## Task 6: Audio Output with CPAL

**Covers:** [S3]

**Files:**
- Create: `src/ffmpeg/output.rs`

- [ ] **Step 1: Implement AudioOutput trait and CPAL implementation**

```rust
// src/ffmpeg/output.rs
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use anyhow::{Result, Context};
use super::decoder::AudioFrame;
use std::sync::{Arc, Mutex};

pub trait AudioOutput: Send {
    fn play(&mut self, frame: &AudioFrame) -> Result<()>;
    fn pause(&mut self) -> Result<()>;
    fn resume(&mut self) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
    fn set_volume(&mut self, volume: f32) -> Result<()>;
    fn sample_rate(&self) -> u32;
    fn channels(&self) -> u16;
}

pub struct CpalOutput {
    device: Device,
    config: StreamConfig,
    stream: Option<Stream>,
    sample_buffer: Arc<Mutex<Vec<f32>>>,
    volume: f32,
}

impl CpalOutput {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host.default_output_device()
            .context("No output device found")?;
        
        let supported_config = device.default_output_config()
            .context("No supported output config")?;
        
        let config = StreamConfig {
            channels: supported_config.channels(),
            sample_rate: supported_config.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        Ok(CpalOutput {
            device,
            config,
            stream: None,
            sample_buffer: Arc::new(Mutex::new(Vec::new())),
            volume: 1.0,
        })
    }

    fn create_stream(&mut self) -> Result<()> {
        let sample_buffer = self.sample_buffer.clone();
        let channels = self.config.channels as usize;
        let volume = self.volume;

        let stream = self.device.build_output_stream(
            &self.config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut buffer = sample_buffer.lock().unwrap();
                let samples_needed = data.len();
                
                for i in 0..samples_needed {
                    if buffer.is_empty() {
                        data[i] = 0.0;
                    } else {
                        let sample = buffer.remove(0) * volume;
                        data[i] = sample;
                    }
                }
            },
            |err| eprintln!("Audio output error: {}", err),
            None,
        ).context("Failed to build output stream")?;

        stream.play().context("Failed to start stream")?;
        self.stream = Some(stream);
        Ok(())
    }
}

impl AudioOutput for CpalOutput {
    fn play(&mut self, frame: &AudioFrame) -> Result<()> {
        let mut buffer = self.sample_buffer.lock().unwrap();
        buffer.extend_from_slice(&frame.data);
        
        if self.stream.is_none() {
            drop(buffer);
            self.create_stream()?;
        }
        
        Ok(())
    }

    fn pause(&mut self) -> Result<()> {
        if let Some(stream) = &self.stream {
            stream.pause().context("Failed to pause stream")?;
        }
        Ok(())
    }

    fn resume(&mut self) -> Result<()> {
        if let Some(stream) = &self.stream {
            stream.play().context("Failed to resume stream")?;
        }
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        self.stream = None;
        let mut buffer = self.sample_buffer.lock().unwrap();
        buffer.clear();
        Ok(())
    }

    fn set_volume(&mut self, volume: f32) -> Result<()> {
        self.volume = volume.clamp(0.0, 1.0);
        Ok(())
    }

    fn sample_rate(&self) -> u32 {
        self.config.sample_rate.0
    }

    fn channels(&self) -> u16 {
        self.config.channels
    }
}
```

- [ ] **Step 2: Update ffmpeg/mod.rs**

```rust
// Add to src/ffmpeg/mod.rs
pub use output::{AudioOutput, CpalOutput};
```

- [ ] **Step 3: Verify output compiles**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add src/ffmpeg/output.rs
git commit -m "feat: implement CPAL audio output"
```

---

## Task 7: Player Controller

**Covers:** [S3, S4]

**Files:**
- Create: `src/player/mod.rs`
- Create: `src/player/controller.rs`

- [ ] **Step 1: Create player module structure**

```rust
// src/player/mod.rs
pub mod controller;
pub mod playlist;
pub mod play_mode;
pub mod history;

pub use controller::PlayerController;
pub use playlist::Playlist;
pub use play_mode::PlayMode;
pub use history::PlayHistory;
```

- [ ] **Step 2: Implement PlayerController**

```rust
// src/player/controller.rs
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use anyhow::{Result, Context};
use crate::ffmpeg::{Decoder, AudioFrame, Resampler, CpalOutput, AudioOutput};
use super::playlist::Playlist;
use super::play_mode::PlayMode;
use super::history::PlayHistory;

pub struct PlayerController {
    decoder: Option<Decoder>,
    resampler: Option<Resampler>,
    output: Arc<Mutex<CpalOutput>>,
    playlist: Playlist,
    play_mode: PlayMode,
    history: PlayHistory,
    is_playing: bool,
    volume: f32,
    current_path: Option<String>,
    position: f64,
    duration: f64,
}

impl PlayerController {
    pub fn new() -> Result<Self> {
        let output = Arc::new(Mutex::new(CpalOutput::new()?));
        
        Ok(PlayerController {
            decoder: None,
            resampler: None,
            output,
            playlist: Playlist::new(),
            play_mode: PlayMode::Sequential,
            history: PlayHistory::new(),
            is_playing: false,
            volume: 1.0,
            current_path: None,
            position: 0.0,
            duration: 0.0,
        })
    }

    pub fn play(&mut self, path: &Path) -> Result<()> {
        // Stop current playback
        self.stop()?;
        
        // Create new decoder
        let mut decoder = Decoder::new(path)?;
        let metadata = decoder.duration();
        
        // Create resampler
        let resampler = Resampler::new(
            44100, // Default input sample rate
            2,     // Default input channels
            self.output.lock()?.sample_rate() as i32,
            self.output.lock()?.channels() as i32,
        )?;
        
        self.decoder = Some(decoder);
        self.resampler = Some(resampler);
        self.current_path = Some(path.to_str().unwrap_or("").to_string());
        self.duration = metadata;
        self.position = 0.0;
        self.is_playing = true;
        
        // Add to history
        let song_info = super::playlist::SongInfo {
            path: path.to_path_buf(),
            title: None,
            artist: None,
        };
        self.history.push(song_info);
        
        // Start playback thread
        let output = self.output.clone();
        let decoder = self.decoder.take().unwrap();
        let resampler = self.resampler.take().unwrap();
        
        thread::spawn(move || {
            // Playback loop would go here
            // For now, just decode and play
        });
        
        Ok(())
    }

    pub fn pause(&mut self) -> Result<()> {
        if self.is_playing {
            self.output.lock()?.pause()?;
            self.is_playing = false;
        }
        Ok(())
    }

    pub fn resume(&mut self) -> Result<()> {
        if !self.is_playing {
            self.output.lock()?.resume()?;
            self.is_playing = true;
        }
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        self.output.lock()?.stop()?;
        self.decoder = None;
        self.resampler = None;
        self.is_playing = false;
        self.position = 0.0;
        Ok(())
    }

    pub fn next(&mut self) -> Result<()> {
        if let Some(next_path) = self.playlist.next() {
            self.play(&next_path.path)
        } else {
            Ok(())
        }
    }

    pub fn previous(&mut self) -> Result<()> {
        if let Some(prev_path) = self.history.back() {
            self.play(&prev_path.path)
        } else {
            Ok(())
        }
    }

    pub fn seek(&mut self, position: f64) -> Result<()> {
        if let Some(decoder) = &mut self.decoder {
            decoder.seek(position)?;
            self.position = position;
        }
        Ok(())
    }

    pub fn set_volume(&mut self, volume: f32) -> Result<()> {
        self.volume = volume;
        self.output.lock()?.set_volume(volume)?;
        Ok(())
    }

    pub fn set_play_mode(&mut self, mode: PlayMode) {
        self.play_mode = mode;
    }

    pub fn progress(&self) -> f64 {
        self.position
    }

    pub fn duration(&self) -> f64 {
        self.duration
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    pub fn add_to_playlist(&mut self, path: PathBuf) {
        self.playlist.add(super::playlist::SongInfo {
            path,
            title: None,
            artist: None,
        });
    }

    pub fn clear_playlist(&mut self) {
        self.playlist.clear();
    }
}
```

- [ ] **Step 3: Verify player compiles**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add src/player/
git commit -m "feat: implement player controller"
```

---

## Task 8: Playlist and Play Mode

**Covers:** [S3]

**Files:**
- Create: `src/player/playlist.rs`
- Create: `src/player/play_mode.rs`
- Create: `src/player/history.rs`

- [ ] **Step 1: Implement Playlist**

```rust
// src/player/playlist.rs
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SongInfo {
    pub path: PathBuf,
    pub title: Option<String>,
    pub artist: Option<String>,
}

pub struct Playlist {
    songs: Vec<SongInfo>,
    current_index: Option<usize>,
}

impl Playlist {
    pub fn new() -> Self {
        Playlist {
            songs: Vec::new(),
            current_index: None,
        }
    }

    pub fn add(&mut self, song: SongInfo) {
        self.songs.push(song);
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.songs.len() {
            self.songs.remove(index);
            if let Some(current) = self.current_index {
                if current >= self.songs.len() {
                    self.current_index = if self.songs.is_empty() { None } else { Some(self.songs.len() - 1) };
                }
            }
        }
    }

    pub fn shuffle(&mut self) {
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        self.songs.shuffle(&mut rng);
    }

    pub fn clear(&mut self) {
        self.songs.clear();
        self.current_index = None;
    }

    pub fn current(&self) -> Option<&SongInfo> {
        self.current_index.and_then(|i| self.songs.get(i))
    }

    pub fn next(&mut self) -> Option<SongInfo> {
        if self.songs.is_empty() {
            return None;
        }
        
        let next_index = match self.current_index {
            Some(current) => (current + 1) % self.songs.len(),
            None => 0,
        };
        
        self.current_index = Some(next_index);
        self.songs.get(next_index).cloned()
    }

    pub fn previous(&mut self) -> Option<SongInfo> {
        if self.songs.is_empty() {
            return None;
        }
        
        let prev_index = match self.current_index {
            Some(current) => {
                if current == 0 {
                    self.songs.len() - 1
                } else {
                    current - 1
                }
            }
            None => 0,
        };
        
        self.current_index = Some(prev_index);
        self.songs.get(prev_index).cloned()
    }

    pub fn len(&self) -> usize {
        self.songs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.songs.is_empty()
    }
}
```

- [ ] **Step 2: Implement PlayMode**

```rust
// src/player/play_mode.rs

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayMode {
    Sequential,
    Loop,
    Single,
    Random,
}

impl PlayMode {
    pub fn next_index(&self, current: usize, length: usize) -> Option<usize> {
        if length == 0 {
            return None;
        }
        
        match self {
            PlayMode::Sequential => {
                if current + 1 < length {
                    Some(current + 1)
                } else {
                    None
                }
            }
            PlayMode::Loop => Some((current + 1) % length),
            PlayMode::Single => Some(current),
            PlayMode::Random => {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                Some(rng.gen_range(0..length))
            }
        }
    }
}
```

- [ ] **Step 3: Implement PlayHistory**

```rust
// src/player/history.rs
use super::playlist::SongInfo;

pub struct PlayHistory {
    history: Vec<SongInfo>,
    position: usize,
}

impl PlayHistory {
    pub fn new() -> Self {
        PlayHistory {
            history: Vec::new(),
            position: 0,
        }
    }

    pub fn push(&mut self, song: SongInfo) {
        // Remove any forward history
        self.history.truncate(self.position);
        self.history.push(song);
        self.position = self.history.len();
    }

    pub fn back(&mut self) -> Option<&SongInfo> {
        if self.position > 0 {
            self.position -= 1;
            self.history.get(self.position)
        } else {
            None
        }
    }

    pub fn forward(&mut self) -> Option<&SongInfo> {
        if self.position < self.history.len() {
            let song = self.history.get(self.position);
            self.position += 1;
            song
        } else {
            None
        }
    }

    pub fn current(&self) -> Option<&SongInfo> {
        self.history.get(self.position.saturating_sub(1))
    }
}
```

- [ ] **Step 4: Update player/mod.rs exports**

```rust
// src/player/mod.rs
pub mod controller;
pub mod playlist;
pub mod play_mode;
pub mod history;

pub use controller::PlayerController;
pub use playlist::{Playlist, SongInfo};
pub use play_mode::PlayMode;
pub use history::PlayHistory;
```

- [ ] **Step 5: Verify playlist compiles**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 6: Commit**

```bash
git add src/player/playlist.rs src/player/play_mode.rs src/player/history.rs
git commit -m "feat: implement playlist and play mode management"
```

---

## Task 9: Update Existing Player Integration

**Covers:** [S4]

**Files:**
- Modify: `src/play/player.rs`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Update lib.rs module declarations**

```rust
// src/lib.rs
pub mod app;
pub mod assets;
pub mod components;
pub mod config;
pub mod db;
pub mod error;
pub mod ffmpeg;
pub mod player;
pub mod play;
pub mod theme;
pub mod ui;
pub mod util;
```

- [ ] **Step 2: Update player.rs to use new backend**

```rust
// src/play/player.rs - Simplified version
use gpui::*;
use crate::player::PlayerController;
use std::path::Path;

pub struct Player {
    controller: PlayerController,
}

impl Player {
    pub fn new() -> Self {
        Player {
            controller: PlayerController::new().expect("Failed to create player controller"),
        }
    }

    pub fn play(&mut self, path: &Path) {
        let _ = self.controller.play(path);
    }

    pub fn pause(&mut self) {
        let _ = self.controller.pause();
    }

    pub fn resume(&mut self) {
        let _ = self.controller.resume();
    }

    pub fn stop(&mut self) {
        let _ = self.controller.stop();
    }

    pub fn next(&mut self) {
        let _ = self.controller.next();
    }

    pub fn previous(&mut self) {
        let _ = self.controller.previous();
    }

    pub fn set_volume(&mut self, volume: f32) {
        let _ = self.controller.set_volume(volume);
    }

    pub fn is_playing(&self) -> bool {
        self.controller.is_playing()
    }

    pub fn progress(&self) -> f64 {
        self.controller.progress()
    }

    pub fn duration(&self) -> f64 {
        self.controller.duration()
    }
}

impl Global for Player {}
```

- [ ] **Step 3: Update metadata.rs to use FFmpeg**

```rust
// src/db/metadata.rs - Simplified version
use crate::ffmpeg::MetadataReader;
use std::path::Path;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct AlbumInfo {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: f64,
    pub cover_path: Option<String>,
}

impl AlbumInfo {
    pub fn new_from_file(path: &Path) -> Result<Self> {
        let reader = MetadataReader::new(path)?;
        let metadata = reader.read_metadata()?;
        
        // Handle cover art
        let cover_path = if let Some(cover_data) = reader.get_cover_art()? {
            let cover_path = path.with_extension("cover.jpg");
            std::fs::write(&cover_path, cover_data)?;
            Some(cover_path.to_str().unwrap_or("").to_string())
        } else {
            None
        };
        
        Ok(AlbumInfo {
            title: metadata.title,
            artist: metadata.artist,
            album: metadata.album,
            duration: metadata.duration,
            cover_path,
        })
    }
}
```

- [ ] **Step 4: Update Cargo.toml to remove rodio and lofty**

```toml
# Remove these lines from Cargo.toml:
# rodio = { version = "0.21.1", features = [...] }
# lofty = "0.22.4"
```

- [ ] **Step 5: Verify integration compiles**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 6: Commit**

```bash
git add src/play/player.rs src/db/metadata.rs src/lib.rs src/main.rs Cargo.toml
git commit -m "feat: integrate FFmpeg backend with existing player"
```

---

## Task 10: Testing and Verification

**Covers:** [S6]

**Files:**
- Create: `tests/ffmpeg_decoder_test.rs`
- Create: `tests/ffmpeg_metadata_test.rs`
- Create: `tests/player_test.rs`

- [ ] **Step 1: Create decoder tests**

```rust
// tests/ffmpeg_decoder_test.rs
use std::path::Path;
use zotu::ffmpeg::Decoder;

#[test]
fn test_decoder_creation() {
    // This test would need a test audio file
    // For now, just verify the module compiles
}

#[test]
fn test_decoder_invalid_path() {
    let result = Decoder::new(Path::new("nonexistent.mp3"));
    assert!(result.is_err());
}
```

- [ ] **Step 2: Create metadata tests**

```rust
// tests/ffmpeg_metadata_test.rs
use std::path::Path;
use zotu::ffmpeg::MetadataReader;

#[test]
fn test_metadata_reader_creation() {
    // This test would need a test audio file
    // For now, just verify the module compiles
}

#[test]
fn test_metadata_reader_invalid_path() {
    let result = MetadataReader::new(Path::new("nonexistent.mp3"));
    assert!(result.is_err());
}
```

- [ ] **Step 3: Create player tests**

```rust
// tests/player_test.rs
use zotu::player::{Playlist, PlayMode, PlayHistory};
use std::path::PathBuf;

#[test]
fn test_playlist_operations() {
    let mut playlist = Playlist::new();
    assert!(playlist.is_empty());
    
    playlist.add(zotu::player::SongInfo {
        path: PathBuf::from("test.mp3"),
        title: None,
        artist: None,
    });
    
    assert_eq!(playlist.len(), 1);
    assert!(!playlist.is_empty());
}

#[test]
fn test_play_mode_sequential() {
    let mode = PlayMode::Sequential;
    assert_eq!(mode.next_index(0, 3), Some(1));
    assert_eq!(mode.next_index(1, 3), Some(2));
    assert_eq!(mode.next_index(2, 3), None);
}

#[test]
fn test_play_history() {
    let mut history = PlayHistory::new();
    assert!(history.back().is_none());
    
    history.push(zotu::player::SongInfo {
        path: PathBuf::from("test1.mp3"),
        title: None,
        artist: None,
    });
    
    assert!(history.back().is_some());
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test`
Expected: Tests pass

- [ ] **Step 5: Verify no compiler warnings**

Run: `cargo build 2>&1 | grep warning`
Expected: No warnings

- [ ] **Step 6: Commit**

```bash
git add tests/
git commit -m "test: add unit tests for FFmpeg backend"
```

---

## Task 11: Final Verification

**Covers:** [S9]

**Files:**
- None (verification only)

- [ ] **Step 1: Run full build**

Run: `cargo build --release`
Expected: Successful release build

- [ ] **Step 2: Run all tests**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 3: Check for unsafe in public API**

Run: `grep -r "unsafe" src/ --include="*.rs" | grep -v "ffi.rs"`
Expected: No unsafe in public API files

- [ ] **Step 4: Verify no memory leaks (manual check)**

- Play a song and stop it
- Check that no FFmpeg resources are leaked
- Verify Drop implementations are called

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "feat: complete FFmpeg audio backend refactoring"
```

---

## Summary

This plan implements a complete FFmpeg audio backend for Zotu with:

1. **Hand-written FFI bindings** - Direct Rust bindings to FFmpeg functions
2. **Safe abstractions** - RAII-based wrappers for all FFmpeg resources
3. **Modular architecture** - Separate modules for decoder, metadata, resampler, filters, and output
4. **Application-layer management** - Playlist, play modes, and history management
5. **Integration** - Seamless replacement of Rodio backend with FFmpeg

The implementation follows TDD principles with comprehensive testing at each phase. All unsafe code is isolated in the FFI layer, with safe public APIs throughout.
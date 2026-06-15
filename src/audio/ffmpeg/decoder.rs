use std::{ffi::CString, marker::PhantomData, os::raw::c_int, path::Path, ptr};

use super::ffi;

#[derive(Clone, Copy)]
pub(super) struct AudioFormat {
    sample_rate: c_int,
    channels: c_int,
    sample_format: c_int,
}

impl AudioFormat {
    pub(super) fn sample_rate(self) -> c_int {
        self.sample_rate
    }

    pub(super) fn channels(self) -> c_int {
        self.channels
    }

    pub(super) fn sample_format(self) -> c_int {
        self.sample_format
    }
}

pub(super) struct DecodedAudioFrame<'a> {
    planes: Vec<*const u8>,
    sample_count: c_int,
    _frame: PhantomData<&'a ffi::AVFrame>,
}

impl DecodedAudioFrame<'_> {
    pub(super) fn planes(&self) -> &[*const u8] {
        &self.planes
    }

    pub(super) fn sample_count(&self) -> c_int {
        self.sample_count
    }
}

struct DecoderResources {
    format_context: *mut ffi::AVFormatContext,
    codec_context: *mut ffi::AVCodecContext,
    frame: *mut ffi::AVFrame,
    packet: *mut ffi::AVPacket,
}

impl DecoderResources {
    fn new() -> Self {
        Self {
            format_context: ptr::null_mut(),
            codec_context: ptr::null_mut(),
            frame: ptr::null_mut(),
            packet: ptr::null_mut(),
        }
    }
}

impl Drop for DecoderResources {
    fn drop(&mut self) {
        unsafe {
            ffi::av_packet_free(&mut self.packet);
            ffi::av_frame_free(&mut self.frame);
            ffi::avcodec_free_context(&mut self.codec_context);
            ffi::avformat_close_input(&mut self.format_context);
        }
    }
}

pub(super) struct FfmpegDecoder {
    resources: DecoderResources,
    stream_index: c_int,
    format: AudioFormat,
    duration_secs: u64,
}

impl FfmpegDecoder {
    pub(super) fn open(path: &Path) -> anyhow::Result<Self> {
        let path = CString::new(path.to_string_lossy().as_bytes())?;
        let mut resources = DecoderResources::new();

        let result = unsafe {
            ffi::avformat_open_input(
                &mut resources.format_context,
                path.as_ptr(),
                ptr::null(),
                ptr::null_mut(),
            )
        };
        if result < 0 {
            return Err(ffi::error(result));
        }

        let result =
            unsafe { ffi::avformat_find_stream_info(resources.format_context, ptr::null_mut()) };
        if result < 0 {
            return Err(ffi::error(result));
        }

        let mut codec = ptr::null();
        let stream_index = unsafe {
            ffi::av_find_best_stream(
                resources.format_context,
                ffi::AVMEDIA_TYPE_AUDIO,
                -1,
                -1,
                &mut codec,
                0,
            )
        };
        if stream_index < 0 {
            return Err(ffi::error(stream_index));
        }
        if codec.is_null() {
            anyhow::bail!("FFmpeg did not return a decoder for the audio stream");
        }

        let codec_parameters = unsafe {
            ffi::zotu_ffmpeg_stream_codec_parameters(resources.format_context, stream_index)
        };
        if codec_parameters.is_null() {
            anyhow::bail!("FFmpeg audio stream has no codec parameters");
        }

        resources.codec_context = unsafe { ffi::avcodec_alloc_context3(codec) };
        if resources.codec_context.is_null() {
            anyhow::bail!("Failed to allocate FFmpeg codec context");
        }

        let result = unsafe {
            ffi::avcodec_parameters_to_context(resources.codec_context, codec_parameters)
        };
        if result < 0 {
            return Err(ffi::error(result));
        }

        let result = unsafe { ffi::avcodec_open2(resources.codec_context, codec, ptr::null_mut()) };
        if result < 0 {
            return Err(ffi::error(result));
        }

        resources.frame = unsafe { ffi::av_frame_alloc() };
        if resources.frame.is_null() {
            anyhow::bail!("Failed to allocate FFmpeg frame");
        }

        resources.packet = unsafe { ffi::av_packet_alloc() };
        if resources.packet.is_null() {
            anyhow::bail!("Failed to allocate FFmpeg packet");
        }

        let format = AudioFormat {
            sample_rate: unsafe { ffi::zotu_ffmpeg_codec_sample_rate(resources.codec_context) },
            channels: unsafe { ffi::zotu_ffmpeg_codec_channels(resources.codec_context) },
            sample_format: unsafe { ffi::zotu_ffmpeg_codec_sample_format(resources.codec_context) },
        };
        if format.sample_rate <= 0 || format.channels <= 0 || format.sample_format < 0 {
            anyhow::bail!(
                "Invalid FFmpeg audio format: sample_rate={}, channels={}, sample_format={}",
                format.sample_rate,
                format.channels,
                format.sample_format
            );
        }

        let duration = unsafe { ffi::zotu_ffmpeg_format_duration(resources.format_context) };
        let duration_secs = if duration > 0 {
            (duration / ffi::AV_TIME_BASE) as u64
        } else {
            0
        };

        Ok(Self {
            resources,
            stream_index,
            format,
            duration_secs,
        })
    }

    pub(super) fn format(&self) -> AudioFormat {
        self.format
    }

    pub(super) fn duration_secs(&self) -> u64 {
        self.duration_secs
    }

    pub(super) fn read_frame(&mut self) -> anyhow::Result<Option<DecodedAudioFrame<'_>>> {
        loop {
            let result = unsafe {
                ffi::avcodec_receive_frame(self.resources.codec_context, self.resources.frame)
            };
            if result >= 0 {
                let sample_count =
                    unsafe { ffi::zotu_ffmpeg_frame_sample_count(self.resources.frame) };
                if sample_count <= 0 {
                    anyhow::bail!("FFmpeg returned an empty audio frame");
                }

                let plane_count =
                    if unsafe { ffi::av_sample_fmt_is_planar(self.format.sample_format) } != 0 {
                        self.format.channels as usize
                    } else {
                        1
                    };
                let mut planes = Vec::with_capacity(plane_count);
                for plane in 0..plane_count {
                    let data = unsafe {
                        ffi::zotu_ffmpeg_frame_data(self.resources.frame, plane as c_int)
                    };
                    if data.is_null() {
                        anyhow::bail!("FFmpeg returned a null audio plane");
                    }
                    planes.push(data);
                }

                return Ok(Some(DecodedAudioFrame {
                    planes,
                    sample_count,
                    _frame: PhantomData,
                }));
            }
            if result == ffi::AVERROR_EOF {
                return Ok(None);
            }
            if result != ffi::AVERROR_EAGAIN {
                return Err(ffi::error(result));
            }

            let result =
                unsafe { ffi::av_read_frame(self.resources.format_context, self.resources.packet) };
            if result == ffi::AVERROR_EOF {
                let flush_result =
                    unsafe { ffi::avcodec_send_packet(self.resources.codec_context, ptr::null()) };
                if flush_result < 0
                    && flush_result != ffi::AVERROR_EOF
                    && flush_result != ffi::AVERROR_EAGAIN
                {
                    return Err(ffi::error(flush_result));
                }
                continue;
            }
            if result < 0 {
                return Err(ffi::error(result));
            }

            let packet_stream =
                unsafe { ffi::zotu_ffmpeg_packet_stream_index(self.resources.packet) };
            let send_result = if packet_stream == self.stream_index {
                unsafe {
                    ffi::avcodec_send_packet(self.resources.codec_context, self.resources.packet)
                }
            } else {
                0
            };
            unsafe { ffi::av_packet_unref(self.resources.packet) };

            if send_result < 0 && send_result != ffi::AVERROR_EAGAIN {
                return Err(ffi::error(send_result));
            }
        }
    }

    pub(super) fn seek(&mut self, position_secs: u64) -> anyhow::Result<()> {
        let timestamp_us = position_secs
            .saturating_mul(ffi::AV_TIME_BASE as u64)
            .min(i64::MAX as u64) as i64;
        let mut timestamp = 0;
        let result = unsafe {
            ffi::zotu_ffmpeg_stream_timestamp_from_us(
                self.resources.format_context,
                self.stream_index,
                timestamp_us,
                &mut timestamp,
            )
        };
        if result < 0 {
            return Err(ffi::error(result));
        }

        let result = unsafe {
            ffi::av_seek_frame(
                self.resources.format_context,
                self.stream_index,
                timestamp,
                ffi::AVSEEK_FLAG_BACKWARD,
            )
        };
        if result < 0 {
            return Err(ffi::error(result));
        }

        unsafe { ffi::avcodec_flush_buffers(self.resources.codec_context) };
        Ok(())
    }
}

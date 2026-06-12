use super::ffi;
use std::ffi::CString;
use std::os::raw::c_int;
use std::path::Path;
use std::ptr;

pub struct AudioFormat {
    pub sample_rate: c_int,
    pub channels: c_int,
    pub sample_fmt: c_int,
}

pub struct FfmpegDecoder {
    fmt_ctx: *mut ffi::AVFormatContext,
    codec_ctx: *mut ffi::AVCodecContext,
    frame: *mut ffi::AVFrame,
    packet: *mut ffi::AVPacket,
    stream_index: c_int,
    pub format: AudioFormat,
    finished: bool,
    duration_secs: u64,
}

unsafe impl Send for FfmpegDecoder {}

impl FfmpegDecoder {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let c_path = CString::new(path.to_string_lossy().as_bytes())?;

        let mut fmt_ctx: *mut ffi::AVFormatContext = ptr::null_mut();
        let ret = unsafe {
            ffi::avformat_open_input(
                &mut fmt_ctx,
                c_path.as_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };
        if ret != 0 {
            return Err(ffi::ffmpeg_error(ret));
        }

        let ret = unsafe { ffi::avformat_find_stream_info(fmt_ctx, ptr::null_mut()) };
        if ret < 0 {
            unsafe { ffi::avformat_close_input(&mut fmt_ctx) };
            return Err(ffi::ffmpeg_error(ret));
        }

        let mut codec_ptr: *const ffi::AVCodec = ptr::null();
        let stream_index = unsafe {
            ffi::av_find_best_stream(
                fmt_ctx,
                1, // AVMEDIA_TYPE_AUDIO
                -1,
                -1,
                &mut codec_ptr as *mut *const ffi::AVCodec,
                0,
            )
        };
        if stream_index < 0 {
            unsafe { ffi::avformat_close_input(&mut fmt_ctx) };
            return Err(anyhow::anyhow!("No audio stream found"));
        }

        let streams_ptr = unsafe { ffi::mimocode_avformat_streams(fmt_ctx) };
        let stream = unsafe { *streams_ptr.add(stream_index as usize) };
        let codecpar = unsafe { ffi::mimocode_avstream_codecpar(stream) };

        let codec_ctx = unsafe { ffi::avcodec_alloc_context3(codec_ptr) };
        if codec_ctx.is_null() {
            unsafe { ffi::avformat_close_input(&mut fmt_ctx) };
            return Err(anyhow::anyhow!("Failed to allocate codec context"));
        }

        let ret = unsafe { ffi::avcodec_parameters_to_context(codec_ctx, codecpar) };
        if ret < 0 {
            unsafe {
                ffi::avcodec_free_context(&mut codec_ctx);
                ffi::avformat_close_input(&mut fmt_ctx);
            }
            return Err(ffi::ffmpeg_error(ret));
        }

        let ret = unsafe { ffi::avcodec_open2(codec_ctx, codec_ptr, ptr::null_mut()) };
        if ret < 0 {
            unsafe {
                ffi::avcodec_free_context(&mut codec_ctx);
                ffi::avformat_close_input(&mut fmt_ctx);
            }
            return Err(ffi::ffmpeg_error(ret));
        }

        let frame = unsafe { ffi::av_frame_alloc() };
        if frame.is_null() {
            unsafe {
                ffi::avcodec_free_context(&mut codec_ctx);
                ffi::avformat_close_input(&mut fmt_ctx);
            }
            return Err(anyhow::anyhow!("Failed to allocate frame"));
        }

        let packet = unsafe { ffi::av_packet_alloc() };
        if packet.is_null() {
            unsafe {
                ffi::av_frame_free(&mut frame);
                ffi::avcodec_free_context(&mut codec_ctx);
                ffi::avformat_close_input(&mut fmt_ctx);
            }
            return Err(anyhow::anyhow!("Failed to allocate packet"));
        }

        let sample_rate = unsafe { ffi::mimocode_avcodec_sample_rate(codec_ctx) };
        let channels = unsafe { ffi::mimocode_avcodec_channels(codec_ctx) };
        let sample_fmt = unsafe { ffi::mimocode_avcodec_sample_fmt(codec_ctx) };

        let duration_secs = unsafe {
            let dur = ffi::mimocode_avformat_duration(fmt_ctx);
            if dur > 0 {
                (dur / 1_000_000) as u64
            } else {
                0
            }
        };

        Ok(Self {
            fmt_ctx,
            codec_ctx,
            frame,
            packet,
            stream_index,
            format: AudioFormat {
                sample_rate,
                channels,
                sample_fmt,
            },
            finished: false,
            duration_secs,
        })
    }

    pub fn duration_secs(&self) -> u64 {
        self.duration_secs
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn read_frame(&mut self) -> anyhow::Result<Option<(*const u8, usize)>> {
        loop {
            let ret = unsafe { ffi::avcodec_receive_frame(self.codec_ctx, self.frame) };
            if ret >= 0 {
                let nb_samples = unsafe { ffi::av_frame_get_nb_samples(self.frame) } as usize;
                let data = unsafe { ffi::av_frame_get_data(self.frame, 0) };
                return Ok(Some((data, nb_samples)));
            }
            if ret == ffi::AVERROR_EOF {
                self.finished = true;
                return Ok(None);
            }
            if ret != -11 {
                return Err(ffi::ffmpeg_error(ret));
            }

            let ret = unsafe { ffi::av_read_frame(self.fmt_ctx, self.packet) };
            if ret == ffi::AVERROR_EOF {
                unsafe { ffi::avcodec_send_packet(self.codec_ctx, ptr::null()) };
                continue;
            }
            if ret < 0 {
                return Err(ffi::ffmpeg_error(ret));
            }

            unsafe {
                if ffi::mimocode_avpacket_stream_index(self.packet) == self.stream_index {
                    let _ = ffi::avcodec_send_packet(self.codec_ctx, self.packet);
                }
                ffi::av_packet_unref(self.packet);
            }
        }
    }

    pub fn seek(&mut self, position_secs: u64) -> anyhow::Result<()> {
        let timestamp = position_secs as i64 * 1_000_000;
        let ret = unsafe {
            ffi::av_seek_frame(self.fmt_ctx, self.stream_index, timestamp as u64, 1)
        };
        if ret < 0 {
            return Err(ffi::ffmpeg_error(ret));
        }
        unsafe {
            ffi::avcodec_flush_buffers(self.codec_ctx);
        }
        self.finished = false;
        Ok(())
    }
}

impl Drop for FfmpegDecoder {
    fn drop(&mut self) {
        unsafe {
            ffi::av_packet_free(&mut self.packet);
            ffi::av_frame_free(&mut self.frame);
            ffi::avcodec_free_context(&mut self.codec_ctx);
            ffi::avformat_close_input(&mut self.fmt_ctx);
        }
    }
}

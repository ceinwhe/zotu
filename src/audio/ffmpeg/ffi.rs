use std::{
    ffi::CStr,
    os::raw::{c_char, c_int, c_longlong},
};

macro_rules! opaque_type {
    ($name:ident) => {
        #[repr(C)]
        pub struct $name {
            _private: [u8; 0],
        }
    };
}

opaque_type!(AVCodec);
opaque_type!(AVCodecContext);
opaque_type!(AVCodecParameters);
opaque_type!(AVDictionary);
opaque_type!(AVFormatContext);
opaque_type!(AVFrame);
opaque_type!(AVInputFormat);
opaque_type!(AVPacket);
opaque_type!(SwrContext);

pub const AVERROR_EAGAIN: c_int = -11;
pub const AVERROR_EOF: c_int = -541478725;
pub const AVMEDIA_TYPE_AUDIO: c_int = 1;
pub const AVSEEK_FLAG_BACKWARD: c_int = 1;
pub const AV_SAMPLE_FMT_FLT: c_int = 3;
pub const AV_TIME_BASE: c_longlong = 1_000_000;

unsafe extern "C" {
    pub fn avformat_open_input(
        context: *mut *mut AVFormatContext,
        filename: *const c_char,
        input_format: *const AVInputFormat,
        options: *mut *mut AVDictionary,
    ) -> c_int;
    pub fn avformat_close_input(context: *mut *mut AVFormatContext);
    pub fn avformat_find_stream_info(
        context: *mut AVFormatContext,
        options: *mut *mut AVDictionary,
    ) -> c_int;
    pub fn av_find_best_stream(
        context: *mut AVFormatContext,
        media_type: c_int,
        wanted_stream: c_int,
        related_stream: c_int,
        decoder: *mut *const AVCodec,
        flags: c_int,
    ) -> c_int;

    pub fn avcodec_alloc_context3(codec: *const AVCodec) -> *mut AVCodecContext;
    pub fn avcodec_free_context(context: *mut *mut AVCodecContext);
    pub fn avcodec_parameters_to_context(
        context: *mut AVCodecContext,
        parameters: *const AVCodecParameters,
    ) -> c_int;
    pub fn avcodec_open2(
        context: *mut AVCodecContext,
        codec: *const AVCodec,
        options: *mut *mut AVDictionary,
    ) -> c_int;
    pub fn avcodec_send_packet(context: *mut AVCodecContext, packet: *const AVPacket) -> c_int;
    pub fn avcodec_receive_frame(context: *mut AVCodecContext, frame: *mut AVFrame) -> c_int;
    pub fn avcodec_flush_buffers(context: *mut AVCodecContext);

    pub fn av_frame_alloc() -> *mut AVFrame;
    pub fn av_frame_free(frame: *mut *mut AVFrame);
    pub fn av_packet_alloc() -> *mut AVPacket;
    pub fn av_packet_free(packet: *mut *mut AVPacket);
    pub fn av_packet_unref(packet: *mut AVPacket);
    pub fn av_read_frame(context: *mut AVFormatContext, packet: *mut AVPacket) -> c_int;
    pub fn av_seek_frame(
        context: *mut AVFormatContext,
        stream_index: c_int,
        timestamp: c_longlong,
        flags: c_int,
    ) -> c_int;

    pub fn av_sample_fmt_is_planar(sample_format: c_int) -> c_int;
    pub fn av_strerror(code: c_int, buffer: *mut c_char, buffer_size: usize) -> c_int;

    pub fn swr_convert(
        context: *mut SwrContext,
        output: *mut *mut u8,
        output_count: c_int,
        input: *const *const u8,
        input_count: c_int,
    ) -> c_int;
    pub fn swr_get_out_samples(context: *mut SwrContext, input_samples: c_int) -> c_int;
    pub fn swr_free(context: *mut *mut SwrContext);

    pub fn zotu_ffmpeg_format_duration(context: *const AVFormatContext) -> c_longlong;
    pub fn zotu_ffmpeg_stream_codec_parameters(
        context: *mut AVFormatContext,
        stream_index: c_int,
    ) -> *mut AVCodecParameters;
    pub fn zotu_ffmpeg_codec_sample_rate(context: *const AVCodecContext) -> c_int;
    pub fn zotu_ffmpeg_codec_sample_format(context: *const AVCodecContext) -> c_int;
    pub fn zotu_ffmpeg_codec_channels(context: *const AVCodecContext) -> c_int;
    pub fn zotu_ffmpeg_packet_stream_index(packet: *const AVPacket) -> c_int;
    pub fn zotu_ffmpeg_frame_sample_count(frame: *const AVFrame) -> c_int;
    pub fn zotu_ffmpeg_frame_data(frame: *const AVFrame, plane: c_int) -> *const u8;
    pub fn zotu_ffmpeg_swr_create(
        context: *mut *mut SwrContext,
        output_channels: c_int,
        output_sample_format: c_int,
        output_sample_rate: c_int,
        input_channels: c_int,
        input_sample_format: c_int,
        input_sample_rate: c_int,
    ) -> c_int;
    pub fn zotu_ffmpeg_stream_timestamp_from_us(
        context: *mut AVFormatContext,
        stream_index: c_int,
        timestamp_us: c_longlong,
        timestamp: *mut c_longlong,
    ) -> c_int;
}

pub fn error(code: c_int) -> anyhow::Error {
    let mut buffer = [0 as c_char; 256];
    let result = unsafe { av_strerror(code, buffer.as_mut_ptr(), buffer.len()) };
    if result < 0 {
        return anyhow::anyhow!("FFmpeg error {code}");
    }

    let message = unsafe { CStr::from_ptr(buffer.as_ptr()) }.to_string_lossy();
    anyhow::anyhow!("FFmpeg error {code}: {message}")
}

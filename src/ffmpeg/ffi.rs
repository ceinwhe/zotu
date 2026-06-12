use std::os::raw::{c_char, c_int, c_uint, c_ulonglong, c_void};

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

pub const AVERROR_EOF: c_int = -541478725;

pub const AV_SAMPLE_FMT_S16: c_int = 1;
pub const AV_SAMPLE_FMT_FLT: c_int = 3;

pub const AV_CH_LAYOUT_STEREO: c_ulonglong = 0x3;
pub const AV_CH_LAYOUT_MONO: c_ulonglong = 0x4;

unsafe extern "C" {
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

    pub fn av_frame_alloc() -> *mut AVFrame;

    pub fn av_frame_free(frame: *mut *mut AVFrame);

    pub fn av_frame_get_nb_samples(frame: *const AVFrame) -> c_int;

    pub fn av_frame_get_channels(frame: *const AVFrame) -> c_int;

    pub fn av_frame_get_sample_rate(frame: *const AVFrame) -> c_int;

    pub fn av_frame_get_linesize(frame: *const AVFrame) -> c_int;

    pub fn av_frame_get_data(frame: *const AVFrame, plane: c_int) -> *mut u8;

    pub fn av_packet_alloc() -> *mut AVPacket;

    pub fn av_packet_free(pkt: *mut *mut AVPacket);

    pub fn av_read_frame(s: *mut AVFormatContext, pkt: *mut AVPacket) -> c_int;

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

    pub fn avcodec_find_decoder(codec_id: c_uint) -> *const AVCodec;

    pub fn avcodec_find_encoder(codec_id: c_uint) -> *const AVCodec;

    pub fn av_strerror(
        errnum: c_int,
        errbuf: *mut c_char,
        errbuf_size: usize,
    ) -> c_int;

    pub fn av_seek_frame(
        s: *mut AVFormatContext,
        stream_index: c_int,
        timestamp: c_ulonglong,
        flags: c_int,
    ) -> c_int;
}

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

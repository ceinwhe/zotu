#include <libavformat/avformat.h>
#include <libavcodec/avcodec.h>
#include <libavutil/frame.h>
#include <libswresample/swresample.h>

int mimocode_avformat_nb_streams(AVFormatContext *ctx) {
    return ctx->nb_streams;
}

AVStream** mimocode_avformat_streams(AVFormatContext *ctx) {
    return ctx->streams;
}

int64_t mimocode_avformat_duration(AVFormatContext *ctx) {
    return ctx->duration;
}

AVCodecParameters* mimocode_avstream_codecpar(AVStream *stream) {
    return stream->codecpar;
}

int mimocode_avcodec_sample_rate(AVCodecContext *ctx) {
    return ctx->sample_rate;
}

int mimocode_avcodec_sample_fmt(AVCodecContext *ctx) {
    return ctx->sample_fmt;
}

int mimocode_avcodec_channels(AVCodecContext *ctx) {
#if LIBAVUTIL_VERSION_MAJOR >= 57
    return ctx->ch_layout.nb_channels;
#else
    return ctx->channels;
#endif
}

int mimocode_avpacket_stream_index(AVPacket *pkt) {
    return pkt->stream_index;
}

#include <errno.h>
#include <stdint.h>

#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavutil/channel_layout.h>
#include <libavutil/error.h>
#include <libavutil/frame.h>
#include <libavutil/mathematics.h>
#include <libavutil/opt.h>
#include <libswresample/swresample.h>

int64_t zotu_ffmpeg_format_duration(const AVFormatContext *context) {
    return context ? context->duration : 0;
}

AVCodecParameters *zotu_ffmpeg_stream_codec_parameters(
    AVFormatContext *context,
    int stream_index) {
    if (!context || stream_index < 0 || stream_index >= (int)context->nb_streams) {
        return NULL;
    }
    return context->streams[stream_index]->codecpar;
}

int zotu_ffmpeg_codec_sample_rate(const AVCodecContext *context) {
    return context ? context->sample_rate : 0;
}

int zotu_ffmpeg_codec_sample_format(const AVCodecContext *context) {
    return context ? context->sample_fmt : AV_SAMPLE_FMT_NONE;
}

int zotu_ffmpeg_codec_channels(const AVCodecContext *context) {
    if (!context) {
        return 0;
    }
#if LIBAVUTIL_VERSION_MAJOR >= 57
    return context->ch_layout.nb_channels;
#else
    return context->channels;
#endif
}

int zotu_ffmpeg_packet_stream_index(const AVPacket *packet) {
    return packet ? packet->stream_index : -1;
}

int zotu_ffmpeg_frame_sample_count(const AVFrame *frame) {
    return frame ? frame->nb_samples : 0;
}

const uint8_t *zotu_ffmpeg_frame_data(const AVFrame *frame, int plane) {
    if (!frame || !frame->extended_data || plane < 0) {
        return NULL;
    }
    return frame->extended_data[plane];
}

int zotu_ffmpeg_swr_create(
    SwrContext **context,
    int out_channels,
    enum AVSampleFormat out_sample_format,
    int out_sample_rate,
    int in_channels,
    enum AVSampleFormat in_sample_format,
    int in_sample_rate) {
    if (!context || out_channels <= 0 || in_channels <= 0 ||
        out_sample_rate <= 0 || in_sample_rate <= 0) {
        return AVERROR(EINVAL);
    }

    *context = NULL;
    SwrContext *resampler = swr_alloc();
    if (!resampler) {
        return AVERROR(ENOMEM);
    }

    int result = 0;
#if LIBAVUTIL_VERSION_MAJOR >= 57
    AVChannelLayout out_layout = {0};
    AVChannelLayout in_layout = {0};
    av_channel_layout_default(&out_layout, out_channels);
    av_channel_layout_default(&in_layout, in_channels);

    result = av_opt_set_chlayout(resampler, "out_chlayout", &out_layout, 0);
    if (result >= 0) {
        result = av_opt_set_chlayout(resampler, "in_chlayout", &in_layout, 0);
    }

    av_channel_layout_uninit(&out_layout);
    av_channel_layout_uninit(&in_layout);
#else
    int64_t out_layout = av_get_default_channel_layout(out_channels);
    int64_t in_layout = av_get_default_channel_layout(in_channels);
    result = av_opt_set_int(resampler, "out_channel_layout", out_layout, 0);
    if (result >= 0) {
        result = av_opt_set_int(resampler, "in_channel_layout", in_layout, 0);
    }
#endif

    if (result >= 0) {
        result = av_opt_set_int(resampler, "out_sample_rate", out_sample_rate, 0);
    }
    if (result >= 0) {
        result = av_opt_set_int(resampler, "in_sample_rate", in_sample_rate, 0);
    }
    if (result >= 0) {
        result = av_opt_set_sample_fmt(resampler, "out_sample_fmt", out_sample_format, 0);
    }
    if (result >= 0) {
        result = av_opt_set_sample_fmt(resampler, "in_sample_fmt", in_sample_format, 0);
    }
    if (result >= 0) {
        result = swr_init(resampler);
    }

    if (result < 0) {
        swr_free(&resampler);
        return result;
    }

    *context = resampler;
    return 0;
}

int zotu_ffmpeg_stream_timestamp_from_us(
    AVFormatContext *context,
    int stream_index,
    int64_t timestamp_us,
    int64_t *timestamp) {
    if (!context || !timestamp || stream_index < 0 ||
        stream_index >= (int)context->nb_streams) {
        return AVERROR(EINVAL);
    }

    AVRational microseconds = {1, AV_TIME_BASE};
    *timestamp = av_rescale_q(
        timestamp_us,
        microseconds,
        context->streams[stream_index]->time_base);
    return 0;
}

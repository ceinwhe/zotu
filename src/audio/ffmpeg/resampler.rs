use super::ffi;
use std::os::raw::c_int;
use std::ptr;

pub struct FfmpegResampler {
    ctx: *mut ffi::SwrContext,
    out_sample_rate: c_int,
    out_channels: c_int,
}

unsafe impl Send for FfmpegResampler {}

impl FfmpegResampler {
    pub fn new(
        in_sample_rate: c_int,
        in_channels: c_int,
        in_sample_fmt: c_int,
        out_sample_rate: c_int,
        out_channels: c_int,
    ) -> anyhow::Result<Self> {
        let out_ch_layout = if out_channels == 2 {
            ffi::AV_CH_LAYOUT_STEREO
        } else {
            ffi::AV_CH_LAYOUT_MONO
        };

        let in_ch_layout = if in_channels == 2 {
            ffi::AV_CH_LAYOUT_STEREO
        } else {
            ffi::AV_CH_LAYOUT_MONO
        };

        let ctx = unsafe {
            ffi::swr_alloc_set_opts(
                ptr::null_mut(),
                out_ch_layout,
                ffi::AV_SAMPLE_FMT_FLT,
                out_sample_rate,
                in_ch_layout,
                in_sample_fmt,
                in_sample_rate,
                0,
                ptr::null_mut(),
            )
        };
        if ctx.is_null() {
            return Err(anyhow::anyhow!("Failed to allocate SwrContext"));
        }

        let ret = unsafe { ffi::swr_init(ctx) };
        if ret < 0 {
            unsafe { ffi::swr_free(&mut (ctx as *mut ffi::SwrContext)) };
            return Err(ffi::ffmpeg_error(ret));
        }

        Ok(Self {
            ctx,
            out_sample_rate,
            out_channels,
        })
    }

    pub fn resample(
        &mut self,
        in_data: &*const u8,
        in_count: c_int,
        out_data: &mut *mut u8,
        out_count: c_int,
    ) -> anyhow::Result<c_int> {
        let written = unsafe {
            ffi::swr_convert(
                self.ctx,
                out_data as *mut *mut u8,
                out_count,
                in_data as *const *const u8,
                in_count,
            )
        };
        if written < 0 {
            return Err(ffi::ffmpeg_error(written));
        }
        Ok(written)
    }

    pub fn out_sample_rate(&self) -> c_int {
        self.out_sample_rate
    }
}

impl Drop for FfmpegResampler {
    fn drop(&mut self) {
        unsafe {
            ffi::swr_free(&mut self.ctx);
        }
    }
}

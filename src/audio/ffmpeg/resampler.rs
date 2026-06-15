use std::{os::raw::c_int, ptr};

use super::{
    decoder::{AudioFormat, DecodedAudioFrame},
    ffi,
};

pub(super) struct ResampledAudio {
    pub(super) samples: Vec<f32>,
    pub(super) frame_count: usize,
}

pub(super) struct FfmpegResampler {
    context: *mut ffi::SwrContext,
    output_sample_rate: c_int,
    output_channels: c_int,
}

impl FfmpegResampler {
    pub(super) fn new(
        input: AudioFormat,
        output_sample_rate: c_int,
        output_channels: c_int,
    ) -> anyhow::Result<Self> {
        let mut context = ptr::null_mut();
        let result = unsafe {
            ffi::zotu_ffmpeg_swr_create(
                &mut context,
                output_channels,
                ffi::AV_SAMPLE_FMT_FLT,
                output_sample_rate,
                input.channels(),
                input.sample_format(),
                input.sample_rate(),
            )
        };
        if result < 0 {
            return Err(ffi::error(result));
        }
        if context.is_null() {
            anyhow::bail!("FFmpeg did not create a resampler context");
        }

        Ok(Self {
            context,
            output_sample_rate,
            output_channels,
        })
    }

    pub(super) fn convert(
        &mut self,
        frame: &DecodedAudioFrame<'_>,
    ) -> anyhow::Result<ResampledAudio> {
        let output_capacity =
            unsafe { ffi::swr_get_out_samples(self.context, frame.sample_count()) };
        if output_capacity < 0 {
            return Err(ffi::error(output_capacity));
        }

        let output_capacity = output_capacity.max(1);
        let mut samples = vec![0.0; output_capacity as usize * self.output_channels as usize];
        let mut output_planes = [samples.as_mut_ptr() as *mut u8];
        let written = unsafe {
            ffi::swr_convert(
                self.context,
                output_planes.as_mut_ptr(),
                output_capacity,
                frame.planes().as_ptr(),
                frame.sample_count(),
            )
        };
        if written < 0 {
            return Err(ffi::error(written));
        }

        samples.truncate(written as usize * self.output_channels as usize);
        Ok(ResampledAudio {
            samples,
            frame_count: written as usize,
        })
    }

    pub(super) fn output_sample_rate(&self) -> c_int {
        self.output_sample_rate
    }
}

impl Drop for FfmpegResampler {
    fn drop(&mut self) {
        unsafe { ffi::swr_free(&mut self.context) };
    }
}

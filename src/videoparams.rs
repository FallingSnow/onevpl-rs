use intel_onevpl_sys as ffi;
use std::{
    mem,
    ops::{Deref, DerefMut},
};

use crate::constants::{ChromaFormat, Codec, FourCC, IoPattern, RateControlMethod, TargetUsage};

#[derive(Copy, Clone, Debug)]
/// See https://spec.oneapi.io/versions/latest/elements/oneVPL/source/API_ref/VPL_structs_cross_component.html#_CPPv413mfxVideoParam for more info.
pub struct VideoParams {
    inner: ffi::mfxVideoParam,
}

impl VideoParams {
    /// Specifies how many asynchronous operations an application performs before the application explicitly synchronizes the result. If zero, the value is not specified.
    pub fn async_depth(&self) -> u16 {
        self.inner.AsyncDepth
    }
    /// If you are running VPP after a decode operation you should be using [`IoPattern::OUT_VIDEO_MEMORY`] on decode params and [`IoPattern::VIDEO_MEMORY`] on this function.
    pub fn set_async_depth(&mut self, depth: u16) {
        self.inner.AsyncDepth = depth;
    }
    /// Input and output memory access types for functions. See the enumerator IOPattern for details. The Query API functions return the natively supported IOPattern if the Query input argument is NULL. This parameter is a mandated input for QueryIOSurf and Init API functions. The output pattern must be specified for DECODE. The input pattern must be specified for ENCODE. Both input and output pattern must be specified for VPP.
    pub fn io_pattern(&self) -> IoPattern {
        IoPattern::from_bits(self.inner.IOPattern).unwrap()
    }
    /// If you are running VPP after a decode operation you should be using [`IoPattern::OUT_VIDEO_MEMORY`] on decode params and [`IoPattern::VIDEO_MEMORY`] on this function.
    pub fn set_io_pattern(&mut self, pattern: IoPattern) {
        self.inner.IOPattern = pattern.bits();
    }
}

impl Default for VideoParams {
    fn default() -> Self {
        Self {
            inner: unsafe { mem::zeroed() },
        }
    }
}

impl Deref for VideoParams {
    type Target = ffi::mfxVideoParam;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for VideoParams {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

// impl TryFrom<Decoder<'_>> for VppVideoParams {
//     type Error = MfxStatus;
//     fn try_from(decoder: Decoder) -> Result<Self, Self::Error> {
//         let mfx_params = decoder.params()?;
//         Ok(VideoParams::from(&mfx_params))
//     }
// }

#[derive(Debug, Clone, Copy, Default)]
/// Configurations related to encoding, decoding, and transcoding. See the definition of the mfxInfoMFX structure for details.
pub struct MfxVideoParams {
    inner: VideoParams,
}

impl MfxVideoParams {
    pub fn set_target_usage(&mut self, usage: TargetUsage) {
        (**self)
            .__bindgen_anon_1
            .mfx
            .__bindgen_anon_1
            .__bindgen_anon_1
            .TargetUsage = usage.repr() as u16;
    }

    pub fn set_initial_delay_in_kb(&mut self, kilobytes: u16) {
        (**self)
            .__bindgen_anon_1
            .mfx
            .__bindgen_anon_1
            .__bindgen_anon_1
            .__bindgen_anon_1
            .InitialDelayInKB = kilobytes;
    }

    pub fn set_qpi(&mut self, qpi: u16) {
        (**self)
            .__bindgen_anon_1
            .mfx
            .__bindgen_anon_1
            .__bindgen_anon_1
            .__bindgen_anon_1
            .QPI = qpi;
    }

    pub fn set_target_kbps(&mut self, kbps: u16) {
        (**self)
            .__bindgen_anon_1
            .mfx
            .__bindgen_anon_1
            .__bindgen_anon_1
            .__bindgen_anon_2
            .TargetKbps = kbps;
    }

    pub fn set_max_kbps(&mut self, kbps: u16) {
        (**self)
            .__bindgen_anon_1
            .mfx
            .__bindgen_anon_1
            .__bindgen_anon_1
            .__bindgen_anon_3
            .MaxKbps = kbps;
    }

    pub fn set_qpp(&mut self, qpp: u16) {
        (**self)
            .__bindgen_anon_1
            .mfx
            .__bindgen_anon_1
            .__bindgen_anon_1
            .__bindgen_anon_2
            .QPP = qpp;
    }

    pub fn set_rate_control_method(&mut self, method: RateControlMethod) {
        (**self)
            .__bindgen_anon_1
            .mfx
            .__bindgen_anon_1
            .__bindgen_anon_1
            .RateControlMethod = method.repr() as u16;
    }

    pub fn set_idr_interval(&mut self, interval: u16) {
        (**self)
            .__bindgen_anon_1
            .mfx
            .__bindgen_anon_1
            .__bindgen_anon_1
            .IdrInterval = interval;
    }

    pub fn set_icq_quality(&mut self, quality: u16) {
        assert!(
            quality >= 1 && quality <= 51,
            "tried to set ICQ quality {quality} outside of inclusive range 1-51"
        );
        (**self)
            .__bindgen_anon_1
            .mfx
            .__bindgen_anon_1
            .__bindgen_anon_1
            .__bindgen_anon_2
            .ICQQuality = quality;
    }

    pub fn set_framerate(&mut self, numerator: u32, denominator: u32) {
        (**self).__bindgen_anon_1.mfx.FrameInfo.FrameRateExtN = numerator;
        (**self).__bindgen_anon_1.mfx.FrameInfo.FrameRateExtD = denominator;
    }

    pub fn set_fourcc(&mut self, format: FourCC) {
        (**self).__bindgen_anon_1.mfx.FrameInfo.FourCC = format.repr();
    }

    pub fn set_chroma_format(&mut self, format: ChromaFormat) {
        (**self).__bindgen_anon_1.mfx.FrameInfo.ChromaFormat = format.repr() as u16;
    }

    pub fn codec(&self) -> Codec {
        Codec::from_repr(unsafe { (**self).__bindgen_anon_1.mfx.CodecId }).unwrap()
    }
    pub fn set_codec(&mut self, codec: Codec) {
        (**self).__bindgen_anon_1.mfx.CodecId = codec as u32;
    }

    pub fn width(&self) -> u16 {
        unsafe {
            (**self)
                .__bindgen_anon_1
                .mfx
                .FrameInfo
                .__bindgen_anon_1
                .__bindgen_anon_1
                .Width
        }
    }
    pub fn set_width(&mut self, width: u16) {
        (**self)
            .__bindgen_anon_1
            .mfx
            .FrameInfo
            .__bindgen_anon_1
            .__bindgen_anon_1
            .Width = width;
    }

    pub fn height(&self) -> u16 {
        unsafe {
            (**self)
                .__bindgen_anon_1
                .mfx
                .FrameInfo
                .__bindgen_anon_1
                .__bindgen_anon_1
                .Height
        }
    }
    pub fn set_height(&mut self, height: u16) {
        (**self)
            .__bindgen_anon_1
            .mfx
            .FrameInfo
            .__bindgen_anon_1
            .__bindgen_anon_1
            .Height = height;
    }

    pub fn set_crop(&mut self, width: u16, height: u16) {
        (**self)
            .__bindgen_anon_1
            .mfx
            .FrameInfo
            .__bindgen_anon_1
            .__bindgen_anon_1
            .CropW = width;
        (**self)
            .__bindgen_anon_1
            .mfx
            .FrameInfo
            .__bindgen_anon_1
            .__bindgen_anon_1
            .CropH = height;
    }

    pub fn crop(&self) -> (u16, u16) {
        unsafe {
            (
                (**self)
                    .__bindgen_anon_1
                    .mfx
                    .FrameInfo
                    .__bindgen_anon_1
                    .__bindgen_anon_1
                    .CropW,
                (**self)
                    .__bindgen_anon_1
                    .mfx
                    .FrameInfo
                    .__bindgen_anon_1
                    .__bindgen_anon_1
                    .CropH,
            )
        }
    }

    /// Returns the maximum size of any compressed frames in bytes.
    pub fn suggested_buffer_size(&self) -> usize {
        unsafe {
            (**self)
                .__bindgen_anon_1
                .mfx
                .__bindgen_anon_1
                .__bindgen_anon_1
                .BufferSizeInKB as usize
                * 1000
        }
    }
}

impl Deref for MfxVideoParams {
    type Target = VideoParams;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for MfxVideoParams {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

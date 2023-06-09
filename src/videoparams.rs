use intel_onevpl_sys as ffi;
use std::{
    mem,
    ops::{Deref, DerefMut},
};

use crate::constants::{ChromaFormat, Codec, FourCC, IoPattern, RateControlMethod, TargetUsage, self};

#[derive(Clone, Debug)]
/// See https://spec.oneapi.io/versions/latest/elements/oneVPL/source/API_ref/VPL_structs_cross_component.html#_CPPv413mfxVideoParam for more info.
/// 
/// This struct requires extra handling when using. In order for the ExtParam value to be set, you must set it with the result of the [`VideoParams::extra_params`] function.
pub struct VideoParams {
    inner: ffi::mfxVideoParam,
    _extra_params: Vec<Box<ExtraCodingOption>>
}

unsafe impl Send for VideoParams {}

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
    // pub fn add_extra_param(&mut self, extra: Box<ExtraCodingOption>) {
    //     self.extra_params.push(extra);
    //     self.inner.NumExtParam = self.extra_params.len() as u16;
    // }
    // pub(crate) fn extra_params(&self) -> Vec<*mut ffi::mfxExtBuffer> {
    //     self.extra_params.iter().map(|x| x as *const _ as *mut _).collect()
    // }
}

impl Default for VideoParams {
    fn default() -> Self {
        Self {
            inner: unsafe { mem::zeroed() },
            _extra_params: Vec::default()
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

#[derive(Debug, Clone, Default)]
/// Configurations related to encoding, decoding, and transcoding. See the definition of the mfxInfoMFX structure for details.
pub struct MfxVideoParams {
    inner: VideoParams,
}

impl MfxVideoParams {
    #[doc = "< Target usage model that guides the encoding process; see the TargetUsage enumerator for details."]
    pub fn set_target_usage(&mut self, usage: TargetUsage) {
        (**self)
            .__bindgen_anon_1
            .mfx
            .__bindgen_anon_1
            .__bindgen_anon_1
            .TargetUsage = usage.repr() as u16;
    }

    #[doc = " Number of pictures within the current GOP (Group of Pictures); if GopPicSize = 0, then the GOP size is unspecified. If GopPicSize = 1, only I-frames are used.\nThe following pseudo-code that shows how the library uses this parameter:\n@code\nmfxU16 get_gop_sequence (...) {\npos=display_frame_order;\nif (pos == 0)\nreturn MFX_FRAMETYPE_I | MFX_FRAMETYPE_IDR | MFX_FRAMETYPE_REF;\n\nIf (GopPicSize == 1) // Only I-frames\nreturn MFX_FRAMETYPE_I | MFX_FRAMETYPE_REF;\n\nif (GopPicSize == 0)\nframeInGOP = pos;    //Unlimited GOP\nelse\nframeInGOP = pos%GopPicSize;\n\nif (frameInGOP == 0)\nreturn MFX_FRAMETYPE_I | MFX_FRAMETYPE_REF;\n\nif (GopRefDist == 1 || GopRefDist == 0)    // Only I,P frames\nreturn MFX_FRAMETYPE_P | MFX_FRAMETYPE_REF;\n\nframeInPattern = (frameInGOP-1)%GopRefDist;\nif (frameInPattern == GopRefDist - 1)\nreturn MFX_FRAMETYPE_P | MFX_FRAMETYPE_REF;\n\nreturn MFX_FRAMETYPE_B;\n}\n@endcode"]
    pub fn set_gop_pic_size(&mut self, size: u16) {
        (**self)
            .__bindgen_anon_1
            .mfx
            .__bindgen_anon_1
            .__bindgen_anon_1
            .GopPicSize = size;
    }

    #[doc = " Distance between I- or P (or GPB) - key frames; if it is zero, the GOP structure is unspecified. Note: If GopRefDist = 1,\nthere are no regular B-frames used (only P or GPB); if mfxExtCodingOption3::GPB is ON, GPB frames (B without backward\nreferences) are used instead of P."]
    pub fn set_gop_ref_dist(&mut self, ref_dist: u16) {
        (**self)
            .__bindgen_anon_1
            .mfx
            .__bindgen_anon_1
            .__bindgen_anon_1
            .GopRefDist = ref_dist;
    }

    #[doc = " Max number of all available reference frames (for AVC/HEVC, NumRefFrame defines DPB size). If NumRefFrame = 0, this parameter is not specified.\nSee also NumRefActiveP, NumRefActiveBL0, and NumRefActiveBL1 in the mfxExtCodingOption3 structure, which set a number of active references."]
    pub fn set_num_ref_frame(&mut self, num: u16) {
        (**self)
            .__bindgen_anon_1
            .mfx
            .__bindgen_anon_1
            .__bindgen_anon_1
            .NumRefFrame = num;
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

    pub fn set_encode_order(&mut self, order: u16) {
        (**self)
            .__bindgen_anon_1
            .mfx
            .__bindgen_anon_1
            .__bindgen_anon_1
            .EncodedOrder = order;
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
        (**self).__bindgen_anon_1.mfx.FrameInfo.FourCC = format.repr() as ffi::mfxU32;
    }

    pub fn set_chroma_format(&mut self, format: ChromaFormat) {
        (**self).__bindgen_anon_1.mfx.FrameInfo.ChromaFormat = format.repr() as u16;
    }

    pub fn codec(&self) -> Codec {
        Codec::from_repr(unsafe { (**self).__bindgen_anon_1.mfx.CodecId } as ffi::_bindgen_ty_14).unwrap()
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

#[derive(Debug, Clone, Copy)]
pub enum ExtraCodingOption {
    ExtraCodingOption1(ExtraCodingOption1),
    ExtraCodingOption2(ExtraCodingOption2),
    ExtraCodingOption3(ExtraCodingOption3),
}

#[derive(Debug, Clone, Copy)]
pub struct ExtraCodingOption1 {
    inner: ffi::mfxExtCodingOption
}

impl Default for ExtraCodingOption1 {
    fn default() -> Self {
        Self {
            inner: unsafe { mem::zeroed() },
        }
    }
}

impl Deref for ExtraCodingOption1 {
    type Target = ffi::mfxExtCodingOption;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for ExtraCodingOption1 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl ExtraCodingOption1 {
    #[doc = "< If set, CAVLC is used; if unset, CABAC is used for encoding. See the CodingOptionValue enumerator for values of this option."]
    pub fn set_cavlc(&mut self, option: constants::CodingOptionValue) {
        (*self).inner.CAVLC = option.repr() as u16;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ExtraCodingOption2 {
    inner: ffi::mfxExtCodingOption2
}

impl Default for ExtraCodingOption2 {
    fn default() -> Self {
        Self {
            inner: unsafe { mem::zeroed() },
        }
    }
}

impl Deref for ExtraCodingOption2 {
    type Target = ffi::mfxExtCodingOption2;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for ExtraCodingOption2 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl ExtraCodingOption2 {
    #[doc = "Controls usage of B-frames as reference. See BRefControl enumerator for values of this option.\nThis parameter is valid only during initialization."]
    pub fn set_b_ref_type(&mut self, control: constants::BRefControl) {
        (*self).inner.BRefType = control.repr() as u16;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ExtraCodingOption3 {
    inner: ffi::mfxExtCodingOption3
}

impl Default for ExtraCodingOption3 {
    fn default() -> Self {
        Self {
            inner: unsafe { mem::zeroed() },
        }
    }
}

impl Deref for ExtraCodingOption3 {
    type Target = ffi::mfxExtCodingOption3;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for ExtraCodingOption3 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl ExtraCodingOption3 {
    #[doc = "< Provides a hint to encoder about the scenario for the encoding session. See the ScenarioInfo enumerator for values of this option."]
    pub fn set_scenario_info(&mut self, info: constants::ScenarioInfo) {
        (*self).inner.ScenarioInfo = info.repr() as u16;
    }
    #[doc = "< Provides a hint to encoder about the content for the encoding session. See the ContentInfo enumerator for values of this option."]
    pub fn set_content_info(&mut self, info: constants::ContentInfo) {
        (*self).inner.ContentInfo = info.repr() as u16;
    }
}
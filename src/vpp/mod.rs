use std::time::Instant;

use ffi::MfxStatus;
use intel_onevpl_sys as ffi;
use std::mem;
use tokio::task;
use tracing::trace;

use crate::{get_library, FrameSurface, Session, decode::Decoder, constants::{FourCC, IoPattern}};

#[derive(Copy, Clone, Debug)]
pub struct VideoParams {
    inner: ffi::mfxVideoParam,
}

impl VideoParams {
    pub fn new() -> Self {
        Self {
            inner: unsafe { mem::zeroed() },
        }
    }
    // pub fn size(&self) -> &ffi::mfxFrameInfo__bindgen_ty_1__bindgen_ty_1 {
    //     unsafe {
    //         &self
    //             .inner
    //             .__bindgen_anon_1
    //             .vpp
    //             .FrameInfo
    //             .__bindgen_anon_1
    //             .__bindgen_anon_1
    //     }
    // }

    /// If you are running VPP after a decode operation you should be using [`IoPattern::OUT_VIDEO_MEMORY`] on decode params and [`IoPattern::VIDEO_MEMORY`] on this function.
    pub fn set_io_pattern(&mut self, pattern: IoPattern) {
        self.inner.IOPattern = pattern.bits();
    }

    pub fn set_fourcc(&mut self, fourcc: FourCC) {
        self.out_mut().FourCC = fourcc.repr();
    }

    /// 23.97 FPS == numerator 24000, denominator = 1001
    pub fn set_framerate(&mut self, numerator: u32, denominator: u32) {
        self.out_mut().FrameRateExtN = numerator;
        self.out_mut().FrameRateExtD = denominator;
    }

    fn in_mut(&mut self) -> &mut ffi::mfxFrameInfo {
        unsafe { &mut self.inner.__bindgen_anon_1.vpp.In }
    }

    fn out_mut(&mut self) -> &mut ffi::mfxFrameInfo {
        unsafe { &mut self.inner.__bindgen_anon_1.vpp.Out }
    }
}

// FIXME: This looks like it's gonna leak memory
impl From<&crate::MFXVideoParams> for VideoParams {
    fn from(value: &crate::MFXVideoParams) -> Self {
        let mut params = Self::new();
        *params.in_mut() = unsafe { value.inner.__bindgen_anon_1.mfx.FrameInfo }.clone();
        *params.out_mut() = unsafe { value.inner.__bindgen_anon_1.mfx.FrameInfo }.clone();
        params
    }
}

impl TryFrom<Decoder<'_>> for VideoParams {
    type Error = MfxStatus;
    fn try_from(decoder: Decoder) -> Result<Self, Self::Error> {
        let mfx_params = decoder.params()?;
        Ok(VideoParams::from(&mfx_params))
    }
}

// pub struct FrameInfo {
//     inner: ffi::mfxFrameInfo,
// }

// impl FrameInfo {
//     pub fn new() -> Self {
//         FrameInfo {
//             inner: unsafe { mem::zeroed() },
//         }
//     }
// }

pub struct VideoProcessor<'a> {
    session: &'a Session,
}

impl<'a> VideoProcessor<'a> {
    pub(crate) fn new(session: &'a Session, params: &mut VideoParams) -> Result<Self, MfxStatus> {
        let lib = get_library().unwrap();
        
        assert!(!IoPattern::from_bits(params.inner.IOPattern).unwrap().is_empty(), "params IOPattern not set");

        let status: MfxStatus =
            unsafe { lib.MFXVideoVPP_Init(session.inner, &mut params.inner) }.into();

        trace!("VPP init = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        let decoder = Self { session };

        Ok(decoder)
    }

    /// The function processes a single input frame to a single output frame
    /// with internal allocation of output frame.
    ///
    /// See
    /// https://spec.oneapi.io/versions/latest/elements/oneVPL/source/API_ref/VPL_func_vid_vpp.html#mfxvideovpp-processframeasync
    /// for more info.
    pub async fn process(
        &self,
        frame: Option<&mut FrameSurface<'_>>,
        timeout: Option<u32>,
    ) -> Result<FrameSurface, MfxStatus> {
        let start_time = Instant::now();
        let lib = get_library().unwrap();

        let input = frame
            .map(|f| f.inner as *mut _)
            .unwrap_or(std::ptr::null_mut());
        let session = self.session.inner;

        let mut output_surface: *mut ffi::mfxFrameSurface1 = std::ptr::null_mut();
        // dbg!(sync_point, output_surface);

        let status: MfxStatus =
            unsafe { lib.MFXVideoVPP_ProcessFrameAsync(session, input, &mut output_surface) }
                .into();

        trace!("Process frame start = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        let mut output_surface = FrameSurface::try_from(output_surface)?;

        let output_surface = task::spawn_blocking(move || {
            output_surface.synchronize(timeout)?;
            Ok(output_surface) as Result<FrameSurface, MfxStatus>
        })
        .await
        .unwrap()?;

        let frame_info = output_surface.inner.Info;
        let format = FourCC::from_repr(frame_info.FourCC).unwrap();
        let height = unsafe { frame_info.__bindgen_anon_1.__bindgen_anon_1.CropH };
        let width = unsafe { frame_info.__bindgen_anon_1.__bindgen_anon_1.CropW };

        trace!("Process frame = {:?} {}x{} {:?}", format, width, height, start_time.elapsed());

        Ok(output_surface)
    }

    /// Stops the current video processing operation and restores internal
    /// structures or parameters for a new operation.
    ///
    /// See
    /// https://spec.oneapi.io/versions/latest/elements/oneVPL/source/API_ref/VPL_func_vid_vpp.html#mfxvideovpp-reset
    /// for more info.
    pub fn reset(&mut self, params: &mut VideoParams) -> Result<(), MfxStatus> {
        let lib = get_library().unwrap();

        let status: MfxStatus =
            unsafe { lib.MFXVideoVPP_Reset(self.session.inner, &mut params.inner) }.into();

        trace!("VPP reset = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        Ok(())
    }

    /// Retrieves current working parameters.
    ///
    /// See
    /// https://spec.oneapi.io/versions/latest/elements/oneVPL/source/API_ref/VPL_func_vid_vpp.html#mfxvideovpp-getvideoparam
    /// for more info.
    pub fn params(&self) -> Result<VideoParams, MfxStatus> {
        let lib = get_library().unwrap();

        let mut params = VideoParams::new();

        let status: MfxStatus =
            unsafe { lib.MFXVideoVPP_GetVideoParam(self.session.inner, &mut params.inner) }.into();

        trace!("VPP get params = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        Ok(params)
    }
}

impl<'a> Drop for VideoProcessor<'a> {
    fn drop(&mut self) {
        let lib = get_library().unwrap();
        unsafe { lib.MFXVideoVPP_Close(self.session.inner) };
    }
}
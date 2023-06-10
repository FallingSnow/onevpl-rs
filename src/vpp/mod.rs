use std::{
    ops::{Deref, DerefMut},
    time::Instant,
};

use ffi::MfxStatus;
use intel_onevpl_sys as ffi;
use tokio::task;
use tracing::trace;

use crate::{
    constants::{ChromaFormat, FourCC, PicStruct},
    get_library,
    videoparams::{MfxVideoParams, VideoParams},
    FrameSurface, Session, utils::SharedPtr,
};

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

pub struct VideoProcessor<'a, 'b: 'a> {
    session: &'a Session<'b>,
}
// unsafe impl Send for VideoProcessor<'_, '_> {}

impl<'a, 'b: 'a> VideoProcessor<'a, 'b> {
    #[tracing::instrument]
    pub(crate) fn new(
        session: &'a Session<'b>,
        params: &mut VppVideoParams,
    ) -> Result<Self, MfxStatus> {
        let lib = get_library().unwrap();

        assert!(!params.io_pattern().is_empty(), "params IOPattern not set");

        let status: MfxStatus =
            unsafe { lib.MFXVideoVPP_Init(session.inner.0, &mut ***params) }.into();

        trace!("VPP init = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        let decoder = Self { session };

        Ok(decoder)
    }

    pub async fn queue(&self,
        frame: Option<&mut FrameSurface<'_>>,
        timeout: Option<u32>,
    ) -> Result<FrameSurface, MfxStatus> {
        let start_time = Instant::now();
        let lib = get_library().unwrap();

        let mut output_surface = SharedPtr(std::ptr::null_mut());
        {
            let input = frame
                .map(|f| f.inner as *mut _)
                .unwrap_or(std::ptr::null_mut());

            let session = self.session.inner.0;

            // dbg!(sync_point, output_surface);

            let status: MfxStatus =
                unsafe { lib.MFXVideoVPP_ProcessFrameAsync(session, input, &mut output_surface.0) }
                    .into();

            trace!("Process frame start = {:?}", status);

            if status != MfxStatus::NoneOrDone {
                return Err(status);
            }
        }

        let output_surface = FrameSurface::try_from(output_surface.0)?;

        let frame_info = output_surface.inner.Info;
        let format = FourCC::from_repr(frame_info.FourCC as ffi::_bindgen_ty_5).unwrap();
        let height = unsafe { frame_info.__bindgen_anon_1.__bindgen_anon_1.CropH };
        let width = unsafe { frame_info.__bindgen_anon_1.__bindgen_anon_1.CropW };

        trace!(
            "Queue frame = {:?} {}x{} {:?}",
            format,
            width,
            height,
            start_time.elapsed()
        );

        Ok(output_surface)
    }

    /// The function processes a single input frame to a single output frame
    /// with internal allocation of output frame.
    ///
    /// See
    /// https://spec.oneapi.io/versions/latest/elements/oneVPL/source/API_ref/VPL_func_vid_vpp.html#mfxvideovpp-processframeasync
    /// for more info.
    // TODO: Use queue function instead of duplicating code
    pub async fn process(
        &self,
        frame: Option<&mut FrameSurface<'_>>,
        timeout: Option<u32>,
    ) -> Result<FrameSurface, MfxStatus> {
        let start_time = Instant::now();
        let lib = get_library().unwrap();

        let mut output_surface = SharedPtr(std::ptr::null_mut());
        {
            let input = frame
                .map(|f| f.inner as *mut _)
                .unwrap_or(std::ptr::null_mut());

            let session = self.session.inner.0;

            // dbg!(sync_point, output_surface);

            let status: MfxStatus =
                unsafe { lib.MFXVideoVPP_ProcessFrameAsync(session, input, &mut output_surface.0) }
                    .into();

            trace!("Process frame start = {:?}", status);

            if status != MfxStatus::NoneOrDone {
                return Err(status);
            }
        }

        let mut output_surface = FrameSurface::try_from(output_surface.0)?;

        let output_surface = task::spawn_blocking(move || {
            output_surface.synchronize(timeout)?;
            Ok(output_surface) as Result<FrameSurface, MfxStatus>
        })
        .await
        .unwrap()?;

        let frame_info = output_surface.inner.Info;
        let format = FourCC::from_repr(frame_info.FourCC as ffi::_bindgen_ty_5).unwrap();
        let height = unsafe { frame_info.__bindgen_anon_1.__bindgen_anon_1.CropH };
        let width = unsafe { frame_info.__bindgen_anon_1.__bindgen_anon_1.CropW };

        trace!(
            "Process frame = {:?} {}x{} {:?}",
            format,
            width,
            height,
            start_time.elapsed()
        );

        Ok(output_surface)
    }

    /// Stops the current video processing operation and restores internal
    /// structures or parameters for a new operation.
    ///
    /// See
    /// https://spec.oneapi.io/versions/latest/elements/oneVPL/source/API_ref/VPL_func_vid_vpp.html#mfxvideovpp-reset
    /// for more info.
    pub fn reset(&mut self, mut params: VppVideoParams) -> Result<(), MfxStatus> {
        let lib = get_library().unwrap();
        let session = self.session.inner.0;

        let status: MfxStatus = unsafe { lib.MFXVideoVPP_Reset(session, &mut **params) }.into();

        trace!("VPP reset = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        Ok(())
    }

    /// Returns surface which can be used as input for VPP.
    ///
    /// See
    /// https://spec.oneapi.io/versions/latest/elements/oneVPL/source/API_ref/VPL_func_mem.html?highlight=getsurfaceforencode#mfxmemory-getsurfaceforvpp
    /// for more info.
    pub fn get_surface_input<'c: 'a>(&mut self) -> Result<FrameSurface<'c>, MfxStatus> {
        let lib = get_library().unwrap();
        let session = self.session.inner.0;

        let mut raw_surface: *mut ffi::mfxFrameSurface1 = std::ptr::null_mut();

        let status: MfxStatus =
            unsafe { lib.MFXMemory_GetSurfaceForVPP(session, &mut raw_surface) }.into();

        trace!("VPP get input surface = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        let surface = FrameSurface::try_from(raw_surface).unwrap();

        Ok(surface)
    }

    /// Returns surface which can be used as output of VPP.  
    ///
    /// See
    /// https://spec.oneapi.io/versions/latest/elements/oneVPL/source/API_ref/VPL_func_mem.html?highlight=getsurfaceforencode#mfxmemory-getsurfaceforvppout
    /// for more info.
    pub fn get_surface_output<'c: 'a>(&mut self) -> Result<FrameSurface<'c>, MfxStatus> {
        let lib = get_library().unwrap();
        let session = self.session.inner.0;

        let mut raw_surface: *mut ffi::mfxFrameSurface1 = std::ptr::null_mut();

        let status: MfxStatus =
            unsafe { lib.MFXMemory_GetSurfaceForVPPOut(session, &mut raw_surface) }.into();

        trace!("VPP get output surface = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        let surface = FrameSurface::try_from(raw_surface).unwrap();

        Ok(surface)
    }

    /// Retrieves current working parameters.
    ///
    /// See
    /// https://spec.oneapi.io/versions/latest/elements/oneVPL/source/API_ref/VPL_func_vid_vpp.html#mfxvideovpp-getvideoparam
    /// for more info.
    pub fn params(&self) -> Result<VppVideoParams, MfxStatus> {
        let lib = get_library().unwrap();
        let session = self.session.inner.0;

        let mut params = VppVideoParams::default();

        let status: MfxStatus =
            unsafe { lib.MFXVideoVPP_GetVideoParam(session, &mut **params) }.into();

        trace!("VPP get params = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        Ok(params)
    }

    /// Verifies VPP support for specified parameters.
    ///
    /// See
    /// https://spec.oneapi.io/versions/latest/elements/oneVPL/source/API_ref/VPL_func_vid_vpp.html#mfxvideovpp-query
    /// for more info.
    pub fn query(
        session: &mut Session,
        input_params: Option<VppVideoParams>,
    ) -> Result<VppVideoParams, (MfxStatus, VppVideoParams)> {
        let lib = get_library().unwrap();
        let session = session.inner.0;

        let mut input_params = input_params.unwrap_or(VppVideoParams::default());

        let mut params = VppVideoParams::default();

        let status: MfxStatus =
            unsafe { lib.MFXVideoVPP_Query(session, &mut **input_params, &mut **params) }.into();

        trace!("VPP query = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err((status, params));
        }

        Ok(params)
    }
}

impl Drop for VideoProcessor<'_, '_> {
    fn drop(&mut self) {
        let lib = get_library().unwrap();
        let session = self.session.inner.0;
        unsafe { lib.MFXVideoVPP_Close(session) };
    }
}

#[derive(Debug, Clone, Default)]
/// Configurations related to video processing. See the definition of the mfxInfoVPP structure for details.
pub struct VppVideoParams {
    inner: VideoParams,
}

impl VppVideoParams {
    pub fn fourcc(&self) -> FourCC {
        FourCC::from_repr(self.out().FourCC as ffi::_bindgen_ty_5).unwrap()
    }

    fn in_(&self) -> &ffi::mfxFrameInfo {
        unsafe { &(*self).__bindgen_anon_1.vpp.In }
    }
    fn in_mut(&mut self) -> &mut ffi::mfxFrameInfo {
        unsafe { &mut (*self).__bindgen_anon_1.vpp.In }
    }

    fn out(&self) -> &ffi::mfxFrameInfo {
        unsafe { &(*self).__bindgen_anon_1.vpp.Out }
    }
    fn out_mut(&mut self) -> &mut ffi::mfxFrameInfo {
        unsafe { &mut (*self).__bindgen_anon_1.vpp.Out }
    }

    pub fn set_in_crop(&mut self, x: u16, y: u16, w: u16, h: u16) {
        self.in_mut().__bindgen_anon_1.__bindgen_anon_1.CropX = x;
        self.in_mut().__bindgen_anon_1.__bindgen_anon_1.CropY = y;
        self.in_mut().__bindgen_anon_1.__bindgen_anon_1.CropW = w;
        self.in_mut().__bindgen_anon_1.__bindgen_anon_1.CropH = h;
    }
    pub fn set_out_crop(&mut self, x: u16, y: u16, w: u16, h: u16) {
        self.out_mut().__bindgen_anon_1.__bindgen_anon_1.CropX = x;
        self.out_mut().__bindgen_anon_1.__bindgen_anon_1.CropY = y;
        self.out_mut().__bindgen_anon_1.__bindgen_anon_1.CropW = w;
        self.out_mut().__bindgen_anon_1.__bindgen_anon_1.CropH = h;
    }

    pub fn set_in_width(&mut self, width: u16) {
        self.in_mut().__bindgen_anon_1.__bindgen_anon_1.Width = width;
    }
    pub fn set_out_width(&mut self, width: u16) {
        self.out_mut().__bindgen_anon_1.__bindgen_anon_1.Width = width;
    }

    pub fn set_in_height(&mut self, height: u16) {
        self.in_mut().__bindgen_anon_1.__bindgen_anon_1.Height = height;
    }
    pub fn set_out_height(&mut self, height: u16) {
        self.out_mut().__bindgen_anon_1.__bindgen_anon_1.Height = height;
    }

    pub fn in_picstruct(&self) -> PicStruct {
        PicStruct::from_repr(self.in_().PicStruct as ffi::_bindgen_ty_6).unwrap()
    }
    pub fn out_picstruct(&self) -> PicStruct {
        PicStruct::from_repr(self.out().PicStruct as ffi::_bindgen_ty_6).unwrap()
    }
    pub fn set_in_picstruct(&mut self, format: PicStruct) {
        self.in_mut().PicStruct = format.repr() as u16;
    }
    pub fn set_out_picstruct(&mut self, format: PicStruct) {
        self.out_mut().PicStruct = format.repr() as u16;
    }

    pub fn in_chroma_format(&self) -> ChromaFormat {
        ChromaFormat::from_repr(self.in_().ChromaFormat as ffi::_bindgen_ty_7).unwrap()
    }
    pub fn out_chroma_format(&self) -> ChromaFormat {
        ChromaFormat::from_repr(self.out().ChromaFormat as ffi::_bindgen_ty_7).unwrap()
    }
    pub fn set_in_chroma_format(&mut self, format: ChromaFormat) {
        self.in_mut().ChromaFormat = format.repr() as u16;
    }
    pub fn set_out_chroma_format(&mut self, format: ChromaFormat) {
        self.out_mut().ChromaFormat = format.repr() as u16;
    }

    pub fn in_fourcc(&self) -> FourCC {
        FourCC::from_repr(self.in_().FourCC as ffi::_bindgen_ty_5).unwrap()
    }
    pub fn out_fourcc(&self) -> FourCC {
        FourCC::from_repr(self.out().FourCC as ffi::_bindgen_ty_5).unwrap()
    }
    pub fn set_in_fourcc(&mut self, fourcc: FourCC) {
        self.in_mut().FourCC = fourcc.repr() as u32;
    }
    pub fn set_out_fourcc(&mut self, fourcc: FourCC) {
        self.out_mut().FourCC = fourcc.repr() as u32;
    }

    /// 23.97 FPS == numerator 24000, denominator = 1001
    pub fn set_in_framerate(&mut self, numerator: u32, denominator: u32) {
        self.in_mut().FrameRateExtN = numerator;
        self.in_mut().FrameRateExtD = denominator;
    }
    pub fn set_out_framerate(&mut self, numerator: u32, denominator: u32) {
        self.out_mut().FrameRateExtN = numerator;
        self.out_mut().FrameRateExtD = denominator;
    }
}

impl Deref for VppVideoParams {
    type Target = VideoParams;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for VppVideoParams {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

// FIXME: This looks like it's gonna be a use after free
impl From<&MfxVideoParams> for VppVideoParams {
    fn from(value: &MfxVideoParams) -> Self {
        let mut params = Self::default();
        *params.in_mut() = unsafe { (**value).__bindgen_anon_1.mfx.FrameInfo }.clone();
        *params.out_mut() = unsafe { (**value).__bindgen_anon_1.mfx.FrameInfo }.clone();
        params
    }
}

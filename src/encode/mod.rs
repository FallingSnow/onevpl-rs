use ffi::MfxStatus;
use intel_onevpl_sys as ffi;
use tokio::task;
use std::{mem, time::Instant};
use tracing::{debug, trace};

use crate::{
    bitstream::Bitstream,
    constants::{FrameType, NalUnitType, SkipFrame},
    get_library,
    videoparams::MfxVideoParams,
    FrameSurface, Session,
};

pub type EncodeStat = ffi::mfxEncodeStat;

#[derive(Debug, Clone, Copy)]
pub struct EncodeCtrl {
    inner: ffi::mfxEncodeCtrl,
}

impl EncodeCtrl {
    pub fn new() -> Self {
        Self {
            inner: unsafe { mem::zeroed() },
        }
    }
    pub fn set_nal_unit_type(&mut self, type_: NalUnitType) {
        self.inner.MfxNalUnitType = type_ as u16;
    }
    pub fn set_skip(&mut self, skip: SkipFrame) {
        self.inner.SkipFrame = skip as u16;
    }
    pub fn set_qp(&mut self, qp: u16) {
        self.inner.QP = qp;
    }
    pub fn set_frame_type(&mut self, type_: FrameType) {
        self.inner.FrameType = type_ as u16;
    }
}

pub struct Encoder<'a> {
    session: &'a mut Session,
    suggested_buffer_size: u16,
}

impl<'a> Encoder<'a> {
    pub fn new(session: &'a mut Session, mut params: MfxVideoParams) -> Result<Self, MfxStatus> {
        let lib = get_library().unwrap();

        let status: MfxStatus =
            unsafe { lib.MFXVideoENCODE_Init(session.inner, &mut **params) }.into();

        trace!("Encode init = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }


        let mut encoder = Self {
            session,
            suggested_buffer_size: 0,
        };

        let params = encoder.params()?;
        encoder.suggested_buffer_size = params.suggested_buffer_size();

        Ok(encoder)
    }

    /// Takes a single input frame in either encoded or display order and generates its output bitstream. Make sure the output buffer is at least the size of params.BufferSizeInKB after you've created a new encoder.
    ///
    /// To mark the end of the encoding sequence, call this function with `input` set to [`None`]. Repeat the call to drain any remaining internally cached bitstreams (one frame at a time) until [`MfxStatus::MoreData`] is returned.
    ///
    /// Returns the number of bytes written to output.
    ///
    /// See https://spec.oneapi.io/versions/latest/elements/oneVPL/source/API_ref/VPL_func_vid_encode.html#mfxvideoencode-encodeframeasync for more info.
    pub async fn encode(
        &mut self,
        controller: &mut EncodeCtrl,
        mut input: Option<FrameSurface<'_>>,
        output: &mut Bitstream<'_>,
        timeout: Option<u32>,
    ) -> Result<usize, MfxStatus> {
        let lib = get_library().unwrap();
        let encode_start = Instant::now();
        let buffer_start_size = output.size();

        if output.len() < self.suggested_buffer_size as usize {
            debug!(
                "WARN: Output buffer is smaller than suggested. {} < {}",
                output.len(),
                self.suggested_buffer_size
            );
        }

        let surface = input.as_mut().map_or(std::ptr::null_mut(), |s| s.inner);

        let mut sync_point: ffi::mfxSyncPoint = std::ptr::null_mut();

        let status: MfxStatus = unsafe {
            lib.MFXVideoENCODE_EncodeFrameAsync(
                self.session.inner,
                &mut controller.inner,
                surface,
                &mut output.inner,
                &mut sync_point,
            )
        }
        .into();
        trace!("Encode frame start = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        task::block_in_place(|| self.session.sync(sync_point, timeout))?;

        trace!("Encoded frame: {:?}", encode_start.elapsed());

        let bytes_written = output.size() - buffer_start_size;
        Ok(bytes_written as usize)
    }

    /// Returns a surface which can be used as input for the encoder.
    ///
    /// See
    /// https://spec.oneapi.io/versions/latest/elements/oneVPL/source/API_ref/VPL_func_mem.html?highlight=getsurfaceforencode#mfxmemory-getsurfaceforencode
    /// for more info.
    pub fn get_surface<'b: 'a>(&mut self) -> Result<FrameSurface<'b>, MfxStatus> {
        let lib = get_library().unwrap();

        let mut raw_surface: *mut ffi::mfxFrameSurface1 = std::ptr::null_mut();

        let status: MfxStatus =
            unsafe { lib.MFXMemory_GetSurfaceForEncode(self.session.inner, &mut raw_surface) }
                .into();

        trace!("Encode get surface = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        let surface = FrameSurface::try_from(raw_surface).unwrap();

        Ok(surface)
    }

    /// Stops the current encoding operation and restores internal structures or parameters for a new encoding operation, possibly with new parameters.
    ///
    /// See https://spec.oneapi.io/versions/latest/elements/oneVPL/source/API_ref/VPL_func_vid_encode.html#mfxvideoencode-reset for more info.
    pub fn reset(&mut self, mut params: MfxVideoParams) -> Result<(), MfxStatus> {
        let lib = get_library().unwrap();

        let status: MfxStatus =
            unsafe { lib.MFXVideoENCODE_Reset(self.session.inner, &mut **params) }.into();

        trace!("Decode reset = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        let params = self.params()?;
        self.suggested_buffer_size = params.suggested_buffer_size();

        Ok(())
    }

    /// Obtains statistics collected during encoding.
    ///
    /// See https://spec.oneapi.io/versions/latest/elements/oneVPL/source/API_ref/VPL_func_vid_encode.html#mfxvideoencode-getencodestat for more info.
    pub fn stats(&mut self) -> Result<EncodeStat, MfxStatus> {
        let lib = get_library().unwrap();

        let mut stats = EncodeStat {
            reserved: [0; 16],
            NumFrame: 0,
            NumBit: 0,
            NumCachedFrame: 0,
        };

        let status: MfxStatus =
            unsafe { lib.MFXVideoENCODE_GetEncodeStat(self.session.inner, &mut stats) }.into();

        trace!("Encode reset = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        Ok(stats)
    }

    /// Retrieves current working parameters.
    ///
    /// See https://spec.oneapi.io/versions/latest/elements/oneVPL/source/API_ref/VPL_func_vid_encode.html#mfxvideoencode-getvideoparam for more info.
    pub fn params(&self) -> Result<MfxVideoParams, MfxStatus> {
        let lib = get_library().unwrap();

        let mut params = MfxVideoParams::default();

        let status: MfxStatus =
            unsafe { lib.MFXVideoENCODE_GetVideoParam(self.session.inner, &mut **params) }.into();

        trace!("Encode get params = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        Ok(params)
    }
}

impl<'a> Drop for Encoder<'a> {
    fn drop(&mut self) {
        let lib = get_library().unwrap();
        unsafe { lib.MFXVideoENCODE_Close(self.session.inner) };
    }
}

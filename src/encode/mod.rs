use ffi::MfxStatus;
use intel_onevpl_sys as ffi;
use std::{mem, time::Instant};
use tokio::task;
use tracing::{trace, warn};

use crate::{
    bitstream::Bitstream,
    constants::{FrameType, NalUnitType, SkipFrame},
    get_library, FrameSurface, MFXVideoParams, Session,
};

pub type EncodeStat = ffi::mfxEncodeStat;

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
    session: &'a Session,
    suggested_buffer_size: u16,
}

impl<'a> Encoder<'a> {
    pub fn new(
        session: &'a Session,
        params: &mut MFXVideoParams,
    ) -> Result<Self, MfxStatus> {
        let lib = get_library().unwrap();

        let status: MfxStatus =
            unsafe { lib.MFXVideoENCODE_Init(session.inner, &mut params.inner) }.into();

        trace!("Encode init = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        let suggested_buffer_size = unsafe {
            params
                .inner
                .__bindgen_anon_1
                .mfx
                .__bindgen_anon_1
                .__bindgen_anon_1
                .BufferSizeInKB
        };

        let encoder = Self {
            session,
            suggested_buffer_size,
        };

        Ok(encoder)
    }

    /// Takes a single input frame in either encoded or display order and generates its output bitstream. Make sure the output buffer is at least the size of params.BufferSizeInKB after you've created a new encoder.
    ///
    /// To mark the end of the encoding sequence, call this function with a NULL surface pointer. Repeat the call to drain any remaining internally cached bitstreams (one frame at a time) until MFX_ERR_MORE_DATA is returned.
    ///
    /// See https://spec.oneapi.io/versions/latest/elements/oneVPL/source/API_ref/VPL_func_vid_encode.html#mfxvideoencode-encodeframeasync for more info.
    pub async fn encode(
        &mut self,
        controller: &mut EncodeCtrl,
        input: Option<&mut FrameSurface<'_>>,
        output: &mut Bitstream<'_>,
        timeout: Option<u32>,
    ) -> Result<(), MfxStatus> {
        let lib = get_library().unwrap();

        if output.len() < self.suggested_buffer_size as usize {
            warn!("Output buffer is smaller than suggested. {} < {}", output.len(), self.suggested_buffer_size);
        }

        let encode_start = Instant::now();

        let mut surface = {
            let mut sync_point: ffi::mfxSyncPoint = std::ptr::null_mut();
            let surface = input.map_or(std::ptr::null_mut(), |s| s.inner);

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

            FrameSurface::try_from(surface)?
        };

        task::spawn_blocking(move || {
            surface.synchronize(timeout)?;
            Ok(surface) as Result<FrameSurface, MfxStatus>
        })
        .await
        .unwrap()?;

        trace!("Encoded frame: {:?}", encode_start.elapsed());

        Ok(())
    }

    /// Stops the current encoding operation and restores internal structures or parameters for a new encoding operation, possibly with new parameters.
    ///
    /// See https://spec.oneapi.io/versions/latest/elements/oneVPL/source/API_ref/VPL_func_vid_encode.html#mfxvideoencode-reset for more info.
    pub fn reset(&mut self, params: &mut MFXVideoParams) -> Result<(), MfxStatus> {
        let lib = get_library().unwrap();

        let status: MfxStatus =
            unsafe { lib.MFXVideoENCODE_Reset(self.session.inner, &mut params.inner) }.into();

        trace!("Decode reset = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

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
    pub fn params(&self) -> Result<MFXVideoParams, MfxStatus> {
        let lib = get_library().unwrap();

        let mut params = MFXVideoParams::new();

        let status: MfxStatus =
            unsafe { lib.MFXVideoENCODE_GetVideoParam(self.session.inner, &mut params.inner) }
                .into();

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

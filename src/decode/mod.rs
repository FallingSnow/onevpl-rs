use ffi::MfxStatus;
use intel_onevpl_sys as ffi;
use std::time::Instant;
use tokio::task;
use tracing::trace;

use crate::{
    bitstream::Bitstream,
    constants::{FourCC, SkipMode},
    get_library, FrameSurface, Session, videoparams::MfxVideoParams,
};

pub struct Decoder<'a: 'b, 'b> {
    session: &'a Session<'b>,
}

impl<'a: 'b, 'b> Decoder<'a, 'b> {
    #[tracing::instrument]
    pub fn new(
        session: &'a Session<'b>,
        mut params: MfxVideoParams,
    ) -> Result<Self, MfxStatus> {
        let lib = get_library().unwrap();

        let status: MfxStatus =
            unsafe { lib.MFXVideoDECODE_Init(session.inner.0, &mut **params) }.into();

        trace!("Decode init = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        let decoder = Self { session };

        Ok(decoder)
    }

    // fn queue_decode(
    //     &self,
    //     bitstream: Option<&mut Bitstream<'_>>,
    // ) -> Result<FrameSurface, MfxStatus> {
    //     let lib = get_library().unwrap();

    //     // If bitstream is null than we are draining
    //     let bitstream = if let Some(bitstream) = bitstream {
    //         &mut bitstream.inner
    //     } else {
    //         std::ptr::null_mut()
    //     };

    //     let mut sync_point: ffi::mfxSyncPoint = std::ptr::null_mut();
    //     let surface_work = std::ptr::null_mut();
    //     let session = self.session.inner;

    //     let mut output_surface: *mut ffi::mfxFrameSurface1 = std::ptr::null_mut();
    //     // dbg!(sync_point, output_surface);

    //     let status: MfxStatus = unsafe {
    //         lib.MFXVideoDECODE_DecodeFrameAsync(
    //             session,
    //             bitstream,
    //             surface_work,
    //             &mut output_surface,
    //             &mut sync_point,
    //         )
    //     }
    //     .into();

    //     trace!("Decode frame start = {:?}", status);

    //     if status != MfxStatus::NoneOrDone {
    //         return Err(status);
    //     }

    //     let output_surface = FrameSurface::try_from(output_surface)?;

    //     Ok(output_surface)
    // }

    /// Decodes the input bitstream to a single output frame. This async
    /// function automatically calls synchronize to wait for the frame to be
    /// decoded.
    ///
    /// See
    /// https://spec.oneapi.io/versions/latest/elements/oneVPL/source/API_ref/VPL_func_vid_decode.html#mfxvideodecode-decodeframeasync
    /// for more info.
    pub async fn decode(
        &self,
        bitstream: Option<&mut Bitstream<'_>>,
        timeout: Option<u32>,
    ) -> Result<FrameSurface, MfxStatus> {
        let decode_start = Instant::now();

        // FIXME: All this is really just a call to queue_decode but I can't get it to compile
        let mut output_surface = {
            let lib = get_library().unwrap();

            // If bitstream is null than we are draining
            let bitstream = if let Some(bitstream) = bitstream {
                &mut bitstream.inner
            } else {
                std::ptr::null_mut()
            };

            let mut sync_point: ffi::mfxSyncPoint = std::ptr::null_mut();
            let surface_work = std::ptr::null_mut();
            let session = self.session.inner.0;

            let mut output_surface: *mut ffi::mfxFrameSurface1 = std::ptr::null_mut();
            // dbg!(sync_point, output_surface);

            let status: MfxStatus = unsafe {
                lib.MFXVideoDECODE_DecodeFrameAsync(
                    session,
                    bitstream,
                    surface_work,
                    &mut output_surface,
                    &mut sync_point,
                )
            }
            .into();

            trace!("Decode frame start = {:?}", status);

            if status != MfxStatus::NoneOrDone {
                return Err(status);
            }

            FrameSurface::try_from(output_surface)?
        };

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
            "Decoded frame = {:?} {}x{} {:?}",
            format,
            width,
            height,
            decode_start.elapsed()
        );

        Ok(output_surface)
    }

    pub fn surface(&mut self) -> Result<FrameSurface, MfxStatus> {
        let lib = get_library().unwrap();
        let session = self.session.inner.0;

        let mut surface = std::ptr::null_mut();

        let status: MfxStatus =
            unsafe { lib.MFXMemory_GetSurfaceForDecode(session, &mut surface) }.into();

        // dbg!(sync_point, output_surface);

        trace!("Get decode surface = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        let surface = FrameSurface::try_from(surface)?;

        Ok(surface)
    }

    /// The application may use this API function to increase decoding performance by sacrificing output quality.
    ///
    /// See https://spec.oneapi.io/versions/latest/elements/oneVPL/source/API_ref/VPL_func_vid_decode.html#mfxvideodecode-setskipmode for more info.
    pub fn set_skip(&mut self, mode: SkipMode) -> Result<(), MfxStatus> {
        let lib = get_library().unwrap();
        let session = self.session.inner.0;

        let status: MfxStatus =
            unsafe { lib.MFXVideoDECODE_SetSkipMode(session, mode.repr()) }.into();

        // dbg!(sync_point, output_surface);

        trace!("Decode frame start = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        Ok(())
    }

    /// Stops the current decoding operation and restores internal structures or
    /// parameters for a new decoding operation.
    ///
    /// Reset serves two purposes:
    /// * It recovers the decoder from errors.
    /// * It restarts decoding from a new position
    ///
    /// See https://spec.oneapi.io/versions/latest/elements/oneVPL/source/API_ref/VPL_func_vid_decode.html#mfxvideodecode-reset for more info.
    pub fn reset(&mut self, mut params: MfxVideoParams) -> Result<(), MfxStatus> {
        let lib = get_library().unwrap();
        let session = self.session.inner.0;

        let status: MfxStatus =
            unsafe { lib.MFXVideoDECODE_Reset(session, &mut **params) }.into();

        trace!("Decode reset = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        Ok(())
    }

    /// Retrieves current working parameters.
    ///
    /// See https://spec.oneapi.io/versions/latest/elements/oneVPL/source/API_ref/VPL_func_vid_decode.html#mfxvideodecode-getvideoparam for more info.
    pub fn params(&self) -> Result<MfxVideoParams, MfxStatus> {
        let lib = get_library().unwrap();
        let session = self.session.inner.0;

        let mut params = MfxVideoParams::default();

        let status: MfxStatus =
            unsafe { lib.MFXVideoDECODE_GetVideoParam(session, &mut **params) }
                .into();

        trace!("Decode get params = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        Ok(params)
    }
}

impl Drop for Decoder<'_, '_> {
    fn drop(&mut self) {
        let lib = get_library().unwrap();
            let session = self.session.inner.0;
            unsafe { lib.MFXVideoDECODE_Close(session) };
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use tracing_test::traced_test;

    use crate::{Loader, constants::{ImplementationType, ApiVersion, Codec, IoPattern}, bitstream::Bitstream};
    
    const DEFAULT_BUFFER_SIZE: usize = 1024 * 1024 * 2; // 2MB

    #[traced_test]
    #[tokio::test]
    async fn decode_hevc_file_frame() {
        // Open file to read from
        let file = std::fs::File::open("tests/frozen.hevc").unwrap();

        let mut loader = Loader::new().unwrap();

        let config = loader.new_config().unwrap();
        // Set software decoding
        config
            .set_filter_property("mfxImplDescription.Impl", ImplementationType::SOFTWARE, None)
            .unwrap();

        let config = loader.new_config().unwrap();
        // Set decode HEVC
        config
            .set_filter_property(
                "mfxImplDescription.mfxDecoderDescription.decoder.CodecID",
                Codec::HEVC,
                None,
            )
            .unwrap();

        let config = loader.new_config().unwrap();
        // Set required API version to 2.2
        config
            .set_filter_property(
                "mfxImplDescription.ApiVersion.Version",
                ApiVersion::new(2, 2),
                None,
            )
            .unwrap();

        let session = loader.new_session(0).unwrap();

        let mut buffer: Vec<u8> = vec![0; DEFAULT_BUFFER_SIZE];
        let mut bitstream = Bitstream::with_codec(&mut buffer, Codec::HEVC);
        let free_buffer_len = (bitstream.len() - bitstream.size() as usize) as u64;
        let bytes_read =
            io::copy(&mut io::Read::take(file, free_buffer_len), &mut bitstream).unwrap();
        assert_ne!(bytes_read, 0);

        let params = session
            .decode_header(&mut bitstream, IoPattern::OUT_SYSTEM_MEMORY)
            .unwrap();

        let decoder = session.decoder(params).unwrap();

        let _frame = decoder.decode(Some(&mut bitstream), None).await.unwrap();
    }

    #[traced_test]
    #[tokio::test]
    async fn decode_hevc_file_video() {
        // Open file to read from
        let mut file = std::fs::File::open("tests/frozen.hevc").unwrap();

        let mut loader = Loader::new().unwrap();

        let config = loader.new_config().unwrap();
        // Set software decoding
        config
            .set_filter_property("mfxImplDescription.Impl", ImplementationType::SOFTWARE, None)
            .unwrap();

        let config = loader.new_config().unwrap();
        // Set decode HEVC
        config
            .set_filter_property(
                "mfxImplDescription.mfxDecoderDescription.decoder.CodecID",
                Codec::HEVC,
                None,
            )
            .unwrap();

        let config = loader.new_config().unwrap();
        // Set required API version to 2.2
        config
            .set_filter_property(
                "mfxImplDescription.ApiVersion.Version",
                ApiVersion::new(2, 2),
                None,
            )
            .unwrap();

        let session = loader.new_session(0).unwrap();

        let mut buffer: Vec<u8> = vec![0; DEFAULT_BUFFER_SIZE];
        let mut bitstream = Bitstream::with_codec(&mut buffer, Codec::HEVC);
        let free_buffer_len = (bitstream.len() - bitstream.size() as usize) as u64;
        let bytes_read = io::copy(
            &mut io::Read::take(&mut file, free_buffer_len),
            &mut bitstream,
        )
        .unwrap();
        assert_ne!(bytes_read, 0);

        let params = session
            .decode_header(&mut bitstream, IoPattern::OUT_SYSTEM_MEMORY)
            .unwrap();

        let decoder = session.decoder(params).unwrap();

        loop {
            let free_buffer_len = (bitstream.len() - bitstream.size() as usize) as u64;
            let bytes_read = io::copy(
                &mut io::Read::take(&mut file, free_buffer_len),
                &mut bitstream,
            )
            .unwrap();

            let _frame = decoder.decode(Some(&mut bitstream), None).await.unwrap();

            if bytes_read == 0 {
                break;
            }
        }
    }

    #[traced_test]
    #[tokio::test]
    async fn decode_hevc_1080p_file_frame() {
        // Open file to read from
        let file = std::fs::File::open("tests/frozen1080.hevc").unwrap();

        let mut loader = Loader::new().unwrap();

        let config = loader.new_config().unwrap();
        // Set software decoding
        config
            .set_filter_property("mfxImplDescription.Impl", ImplementationType::SOFTWARE, None)
            .unwrap();

        let config = loader.new_config().unwrap();
        // Set decode HEVC
        config
            .set_filter_property(
                "mfxImplDescription.mfxDecoderDescription.decoder.CodecID",
                Codec::HEVC,
                None,
            )
            .unwrap();

        let config = loader.new_config().unwrap();
        // Set required API version to 2.2
        config
            .set_filter_property(
                "mfxImplDescription.ApiVersion.Version",
                ApiVersion::new(2, 2),
                None,
            )
            .unwrap();

        let session = loader.new_session(0).unwrap();

        let mut buffer: Vec<u8> = vec![0; DEFAULT_BUFFER_SIZE];
        let mut bitstream = Bitstream::with_codec(&mut buffer, Codec::HEVC);
        let free_buffer_len = (bitstream.len() - bitstream.size() as usize) as u64;
        let bytes_read =
            io::copy(&mut io::Read::take(file, free_buffer_len), &mut bitstream).unwrap();
        assert_ne!(bytes_read, 0);

        let params = session
            .decode_header(&mut bitstream, IoPattern::OUT_SYSTEM_MEMORY)
            .unwrap();

        let decoder = session.decoder(params).unwrap();

        let _frame = decoder.decode(Some(&mut bitstream), None).await.unwrap();
    }
}
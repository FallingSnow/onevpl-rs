use std::{mem, ops::Deref};

use ffi::{
    mfxBitstream, mfxConfig, mfxLoader, mfxSession, mfxStructVersion,
    mfxStructVersion__bindgen_ty_1, mfxU32, mfxVariant, mfxVariantType_MFX_VARIANT_TYPE_U32,
    mfxVariant_data, MfxStatus,
};
use intel_onevpl_sys as ffi;

use once_cell::sync::OnceCell;

static LIBRARY: OnceCell<ffi::vpl> = OnceCell::new();

// The loader object remembers all created mfxConfig objects and destroys them during the mfxUnload function call.
pub struct Loader {
    inner: mfxLoader,
    // configs: Configs
}
impl Loader {
    pub fn new() -> Result<Self, MfxStatus> {
        let lib = LIBRARY.get().unwrap();
        let loader = unsafe { lib.MFXLoad() };
        if loader.is_null() {
            return Err(MfxStatus::Unknown);
        }

        Ok(Self { inner: loader })
    }
    pub fn new_config(&mut self) -> Result<Config, MfxStatus> {
        Config::new(self)
    }
    pub fn new_session(&mut self, index: mfxU32) -> Result<Session, MfxStatus> {
        Session::new(self, index)
    }
}

impl Deref for Loader {
    type Target = mfxLoader;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Drop for Loader {
    fn drop(&mut self) {
        let lib = LIBRARY.get().unwrap();
        unsafe { lib.MFXUnload(self.inner) };
    }
}

pub struct Config {
    inner: mfxConfig,
}
impl Config {
    pub(crate) fn new(loader: &mut Loader) -> Result<Self, MfxStatus> {
        let lib = LIBRARY.get().unwrap();
        let config = unsafe { lib.MFXCreateConfig(loader.inner) };
        if config.is_null() {
            return Err(MfxStatus::Unknown);
        }
        return Ok(Self { inner: config });
    }
    pub fn set_filter_property_u32(
        self,
        name: &str,
        value: u32,
        version: Option<mfxStructVersion>,
    ) -> Result<(), MfxStatus> {
        let lib = LIBRARY.get().unwrap();
        let version = version.unwrap_or(mfxStructVersion {
            __bindgen_anon_1: mfxStructVersion__bindgen_ty_1 { Minor: 0, Major: 0 },
        });

        let variant = mfxVariant {
            Version: version,
            Type: mfxVariantType_MFX_VARIANT_TYPE_U32,
            Data: mfxVariant_data { U32: value },
        };

        let status = unsafe {
            lib.MFXSetConfigFilterProperty(self.inner, name.as_bytes().as_ptr(), variant)
        }
        .into();

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        Ok(())
    }
}

pub struct Session {
    inner: mfxSession,
}
impl Session {
    pub(crate) fn new(loader: &mut Loader, index: mfxU32) -> Result<Self, MfxStatus> {
        //
        let lib = LIBRARY.get().unwrap();
        let mut session: mfxSession = unsafe { mem::zeroed() };
        let status: MfxStatus =
            unsafe { lib.MFXCreateSession(loader.inner, index, &mut session) }.into();

        if status == MfxStatus::NoneOrDone {
            return Err(status);
        }

        Ok(Self { inner: session })
    }

    #[cfg(feature = "va")]
    pub fn set_accelerator(&self) -> Result<(), MfxStatus> {
        // let display = libva::Display::open_drm_display("/dev/dri/renderD128").unwrap();
        // let lib = LIBRARY.get().unwrap();
        // let status: MfxStatus = unsafe {
        //     lib.MFXVideoCORE_SetHandle(self.inner, ffi::mfxHandleType_MFX_HANDLE_VA_DISPLAY, display.handle())
        // }
        // .into();

        // if status == MfxStatus::NoneOrDone {
        //     return Err(status);
        // }
        Ok(())
        // if ((impl & MFX_IMPL_VIA_VAAPI) == MFX_IMPL_VIA_VAAPI) {
        //     VADisplay va_dpy = NULL;
        //     int fd;
        //     // initialize VAAPI context and set session handle (req in Linux)
        //     fd = open("/dev/dri/renderD128", O_RDWR);
        //     if (fd >= 0) {
        //         va_dpy = vaGetDisplayDRM(fd);
        //         if (va_dpy) {
        //             int major_version = 0, minor_version = 0;
        //             if (VA_STATUS_SUCCESS == vaInitialize(va_dpy, &major_version, &minor_version)) {
        //                 MFXVideoCORE_SetHandle(session,
        //                                        static_cast<mfxHandleType>(MFX_HANDLE_VA_DISPLAY),
        //                                        va_dpy);
        //             }
        //         }
        //     }
        //     return va_dpy;
        // }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let lib = LIBRARY.get().unwrap();
        unsafe { lib.MFXClose(self.inner) };
    }
}

pub fn init() -> Result<&'static ffi::vpl, libloading::Error> {
    if let Some(vpl) = LIBRARY.get() {
        return Ok(vpl);
    }

    let library_name = libloading::library_filename("vpl");
    let lib = unsafe { ffi::vpl::new(library_name) }?;

    // FIXME: Check for failure (unwrap/expect)
    LIBRARY.set(lib);

    Ok(LIBRARY.get().unwrap())
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum Codec {
    #[doc = "< AVC, H.264, or MPEG-4, part 10 codec."]
    AVC = ffi::MFX_CODEC_AVC,
    #[doc = "< HEVC codec."]
    HEVC = ffi::MFX_CODEC_HEVC,
    #[doc = "< MPEG-2 codec."]
    MPEG2 = ffi::MFX_CODEC_MPEG2,
    #[doc = "< VC-1 codec."]
    VC1 = ffi::MFX_CODEC_VC1,
    #[doc = "<"]
    CAPTURE = ffi::MFX_CODEC_CAPTURE,
    #[doc = "< VP9 codec."]
    VP9 = ffi::MFX_CODEC_VP9,
    #[doc = "< AV1 codec."]
    AV1 = ffi::MFX_CODEC_AV1,
}

pub struct Bitstream<'a> {
    source: &'a [u8],
    bitstream: mfxBitstream,
    size: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum Impl {
    #[doc = "< Pure Software Implementation."]
    Software = ffi::mfxImplType_MFX_IMPL_TYPE_SOFTWARE,
    #[doc = "< Hardware Accelerated Implementation."]
    Hardware = ffi::mfxImplType_MFX_IMPL_TYPE_HARDWARE,
}

impl<'a> Bitstream<'a> {
    pub fn new(source: &'a mut [u8], codec: Codec) -> Self {
        let mut bitstream: mfxBitstream = unsafe { mem::zeroed() };
        bitstream.Data = source.as_mut_ptr();
        bitstream.MaxLength = source.len() as u32;
        bitstream.__bindgen_anon_1.__bindgen_anon_1.CodecId = codec as u32;
        Self {
            source,
            bitstream,
            size: source.len(),
        }
    }
}

pub mod utils {
    pub fn align16(x: u16) -> u16 {
        ((x + 15) >> 4) << 4
    }

    pub fn align32(x: u32) -> u32 {
        (x + 31) & !31
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ffi::{mfxImplType_MFX_IMPL_TYPE_SOFTWARE, MFX_CODEC_HEVC};

    #[test]
    fn create_session() {
        init().unwrap();
        let mut loader = Loader::new().unwrap();

        let config = loader.new_config().unwrap();
        // Set software decoding
        config
            .set_filter_property_u32(
                "mfxImplDescription.Impl",
                mfxImplType_MFX_IMPL_TYPE_SOFTWARE,
                None,
            )
            .unwrap();

        let config = loader.new_config().unwrap();
        // Set decode HEVC
        config
            .set_filter_property_u32(
                "mfxImplDescription.mfxDecoderDescription.decoder.CodecID",
                MFX_CODEC_HEVC,
                None,
            )
            .unwrap();

        let config = loader.new_config().unwrap();
        // Set required API version to 2.2
        config
            .set_filter_property_u32(
                "mfxImplDescription.ApiVersion.Version",
                (2u32 << 16) + 2,
                None,
            )
            .unwrap();

        let session = loader.new_session(0).unwrap();

        // TODO
        // accelHandle = InitAcceleratorHandle(session);
        // let accel_handle = null_mut();
    }

    #[test]
    fn decode_file() {
        init().unwrap();
        let mut loader = Loader::new().unwrap();

        let config = loader.new_config().unwrap();
        // Set software decoding
        config
            .set_filter_property_u32("mfxImplDescription.Impl", Impl::Software as u32, None)
            .unwrap();

        let config = loader.new_config().unwrap();
        // Set decode HEVC
        config
            .set_filter_property_u32(
                "mfxImplDescription.mfxDecoderDescription.decoder.CodecID",
                Codec::HEVC as u32,
                None,
            )
            .unwrap();

        let config = loader.new_config().unwrap();
        // Set required API version to 2.2
        config
            .set_filter_property_u32(
                "mfxImplDescription.ApiVersion.Version",
                (2u32 << 16) + 2,
                None,
            )
            .unwrap();

        let session = loader.new_session(0).unwrap();
    }
}

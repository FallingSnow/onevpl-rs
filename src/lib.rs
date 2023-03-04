use std::ffi::c_void;
use std::fs::File;
use std::time::Instant;
use std::{
    io::{self, Write},
    mem,
    ops::Deref,
};

use constants::{BitstreamDataFlags, Codec, FourCC, Implementation, IoPattern, SkipMode};
use ffi::{
    mfxBitstream, mfxConfig, mfxLoader, mfxSession, mfxStructVersion,
    mfxStructVersion__bindgen_ty_1, mfxU32, mfxVariant, MfxStatus,
};
use intel_onevpl_sys as ffi;

use once_cell::sync::OnceCell;
use tokio::task;
use tracing::{debug, trace};

use crate::constants::MemoryFlag;

// use crate::callback_future::CbFuture;

// mod callback_future;
pub mod constants;

static LIBRARY: OnceCell<ffi::vpl> = OnceCell::new();

// The loader object remembers all created mfxConfig objects and destroys them during the mfxUnload function call.
#[derive(Debug)]
pub struct Loader {
    inner: mfxLoader,
    accelerator: Option<AcceleratorHandle>,
}
impl Loader {
    #[tracing::instrument]
    pub fn new() -> Result<Self, MfxStatus> {
        let lib = get_library().unwrap();
        let loader = unsafe { lib.MFXLoad() };
        if loader.is_null() {
            return Err(MfxStatus::Unknown);
        }
        debug!("New loader created");

        Ok(Self {
            inner: loader,
            accelerator: None,
        })
    }

    pub fn new_config(&mut self) -> Result<Config, MfxStatus> {
        Config::new(self)
    }

    pub fn new_session(&mut self, index: mfxU32) -> Result<Session, MfxStatus> {
        Session::new(self, index)
    }

    /// Usually you want to open `/dev/dri/renderD128` and pass that in a [`AcceleratorHandle::VAAPI`].
    pub fn set_accelerator(&mut self, handle: AcceleratorHandle) -> Result<(), MfxStatus> {
        let config = self.new_config().unwrap();
        config.set_filter_property("mfxHandleType", handle.mfx_type().into(), None)?;
        let config = self.new_config().unwrap();
        config.set_filter_property("mfxHDL", (*handle.handle()).into(), None)?;

        self.accelerator = Some(handle);

        Ok(())
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
        let lib = get_library().unwrap();
        unsafe { lib.MFXUnload(self.inner) };
    }
}

#[derive(Debug, Copy, Clone)]
pub enum FilterProperty {
    I32(i32),
    U32(u32),
    Ptr(*mut std::ffi::c_void),
}
impl FilterProperty {
    pub fn filter_type(&self) -> u32 {
        match self {
            FilterProperty::I32(_) => ffi::mfxVariantType_MFX_VARIANT_TYPE_I32,
            FilterProperty::U32(_) => ffi::mfxVariantType_MFX_VARIANT_TYPE_U32,
            FilterProperty::Ptr(_) => ffi::mfxVariantType_MFX_VARIANT_TYPE_PTR,
        }
    }
    pub(crate) fn data(&self) -> ffi::mfxVariant_data {
        use ffi::mfxVariant_data;
        match *self {
            FilterProperty::I32(value) => mfxVariant_data { I32: value },
            FilterProperty::U32(value) => mfxVariant_data { U32: value },
            FilterProperty::Ptr(value) => mfxVariant_data { Ptr: value },
        }
    }
}

impl From<u32> for FilterProperty {
    fn from(value: u32) -> Self {
        Self::U32(value)
    }
}
impl From<i32> for FilterProperty {
    fn from(value: i32) -> Self {
        Self::I32(value)
    }
}
impl From<*mut std::ffi::c_void> for FilterProperty {
    fn from(value: *mut std::ffi::c_void) -> Self {
        Self::Ptr(value)
    }
}

#[derive(Debug)]
pub struct Config {
    inner: mfxConfig,
}
impl Config {
    #[tracing::instrument]
    pub(crate) fn new(loader: &mut Loader) -> Result<Self, MfxStatus> {
        let lib = get_library().unwrap();
        let config = unsafe { lib.MFXCreateConfig(loader.inner) };
        if config.is_null() {
            return Err(MfxStatus::Unknown);
        }
        return Ok(Self { inner: config });
    }

    #[tracing::instrument]
    pub fn set_filter_property(
        self,
        name: &str,
        value: FilterProperty,
        version: Option<mfxStructVersion>,
    ) -> Result<(), MfxStatus> {
        let lib = get_library().unwrap();
        let version = version.unwrap_or(mfxStructVersion {
            __bindgen_anon_1: mfxStructVersion__bindgen_ty_1 { Minor: 0, Major: 0 },
        });

        let mut name = name.to_string();
        // CStrings need to nul terminated
        name.push('\0');

        let _type = value.filter_type();
        let data = value.data();

        let variant = mfxVariant {
            Version: version,
            Type: _type,
            Data: data,
        };

        let status =
            unsafe { lib.MFXSetConfigFilterProperty(self.inner, name.as_ptr(), variant) }.into();

        debug!(
            "Setting filter property [{} = {:?}] : {:?}",
            name, data, status
        );

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        Ok(())
    }

    // #[tracing::instrument]
    // pub fn set_filter_property_ptr(
    //     self,
    //     name: &str,
    //     value: *mut c_void,
    //     version: Option<mfxStructVersion>,
    // ) -> Result<(), MfxStatus> {
    //     let lib = get_library().unwrap();
    //     let version = version.unwrap_or(mfxStructVersion {
    //         __bindgen_anon_1: mfxStructVersion__bindgen_ty_1 { Minor: 0, Major: 0 },
    //     });

    //     let mut name = name.to_string();
    //     // CStrings need to nul terminated
    //     name.push('\0');

    //     let variant = mfxVariant {
    //         Version: version,
    //         Type: mfxVariantType_MFX_VARIANT_TYPE_PTR,
    //         Data: ffi::mfxVariant_data { Ptr: value },
    //     };

    //     let status =
    //         unsafe { lib.MFXSetConfigFilterProperty(self.inner, name.as_ptr(), variant) }.into();

    //     debug!(
    //         "Setting filter property [{} = {:?}] : {:?}",
    //         name, value, status
    //     );

    //     if status != MfxStatus::NoneOrDone {
    //         return Err(status);
    //     }

    //     Ok(())
    // }
}

pub struct FrameSurface<'a> {
    inner: &'a mut ffi::mfxFrameSurface1,
    read_offset: usize,
}

unsafe impl Send for FrameSurface<'_> {}

impl FrameSurface<'_> {
    /// Guarantees readiness of both the data (pixels) and any frame's meta information (for example corruption flags) after a function completes. See [`ffi::mfxFrameSurfaceInterface::Synchronize`] for more info.
    ///
    /// Setting `timeout` to None defaults to 100 (in milliseconds)
    ///
    /// [`Decoder::decode`] calls this automatically.
    pub fn synchronize(&mut self, timeout: Option<u32>) -> Result<(), MfxStatus> {
        let timeout = timeout.unwrap_or(100);
        let sync_func = self.interface().Synchronize.unwrap();
        let status: MfxStatus = unsafe { sync_func(self.inner, timeout) }.into();

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        Ok(())
    }

    fn interface(&mut self) -> ffi::mfxFrameSurfaceInterface {
        unsafe { *self.inner.__bindgen_anon_1.FrameInterface }
    }

    /// Sets pointers of surface->Info.Data to actual pixel data, providing read-write access. See [`ffi::mfxFrameSurfaceInterface::Map`] for more info.
    fn map(&mut self) -> Result<(), MfxStatus> {
        // Get memory mapping function
        let func = self.interface().Map.unwrap();

        // Map surface data to get read access to it
        let status: MfxStatus = unsafe { func(self.inner, MemoryFlag::READ.bits()) }.into();

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        Ok(())
    }

    /// Invalidates pointers of surface->Info.Data and sets them to NULL. See [`ffi::mfxFrameSurfaceInterface::Unmap`] for more info.
    fn unmap(&mut self) -> Result<(), MfxStatus> {
        // Get memory mapping function
        let func = self.interface().Unmap.unwrap();

        // Map surface data to get read access to it
        let status: MfxStatus = unsafe { func(self.inner) }.into();

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        Ok(())
    }

    /// Decrements the internal reference counter of the surface. See [`ffi::mfxFrameSurfaceInterface::Release`] for more info.
    fn release(&mut self) -> Result<(), MfxStatus> {
        // Get memory mapping function
        let func = self.interface().Release.unwrap();

        // Map surface data to get read access to it
        let status: MfxStatus = unsafe { func(self.inner) }.into();

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        Ok(())
    }
}

impl Drop for FrameSurface<'_> {
    fn drop(&mut self) {
        self.release().unwrap();
    }
}

impl<'a> TryFrom<*mut ffi::mfxFrameSurface1> for FrameSurface<'a> {
    type Error = MfxStatus;

    fn try_from(value: *mut ffi::mfxFrameSurface1) -> Result<Self, Self::Error> {
        let frame_surface = if let Some(frame_surface_ptr) = unsafe { value.as_mut() } {
            Self {
                inner: frame_surface_ptr,
                read_offset: 0,
            }
        } else {
            return Err(MfxStatus::NullPtr);
        };

        Ok(frame_surface)
    }
}

impl io::Read for FrameSurface<'_> {
    fn read(&mut self, mut buf: &mut [u8]) -> std::io::Result<usize> {
        let data: ffi::mfxFrameData = self.inner.Data;
        let info: ffi::mfxFrameInfo = self.inner.Info;

        let h = unsafe { info.__bindgen_anon_1.__bindgen_anon_1.Height } as usize;
        let w = unsafe { info.__bindgen_anon_1.__bindgen_anon_1.Width } as usize;

        // dbg!(w, h);

        // FIXME: Remove unwrap and replace with actual error
        self.map().unwrap();
        let mut bytes_written = 0;

        // FIXME: Remove unwrap and replace with actual error
        'outer: {
            match FourCC::from_repr(info.FourCC).unwrap() {
                FourCC::IyuvOrI420 => {
                    #[cfg(feature = "vector-write")]
                    let mut io_slices: Vec<io::IoSlice> = Vec::with_capacity(h * 2);
                    let pitch = unsafe { data.__bindgen_anon_2.Pitch } as usize;

                    // Y
                    let y_start = self.read_offset / w;
                    let total_y_size = w * h;
                    // dbg!(pitch, w, y_start, h, self.read_offset);
                    for i in y_start..h {
                        let offset = i * pitch;
                        let ptr = unsafe { data.__bindgen_anon_3.Y.offset(offset as isize) };
                        debug_assert!(!ptr.is_null());
                        let slice: &[u8] = unsafe { std::slice::from_raw_parts(ptr, w) };

                        // This vector write implementation is not very good because it gets all the slices (entire frame) even though we might end up only writing a couple slices. So it has a lot of overhead.
                        #[cfg(feature = "vector-write")]
                        {
                            let io_slice = std::io::IoSlice::new(slice);
                            io_slices.push(io_slice);
                        }
                        #[cfg(not(feature = "vector-write"))]
                        {
                            // We don't want to write a portion of a slice, only whole slices
                            let bytes = if slice.len() <= buf.len() {
                                // FIXME: remove unwrap
                                buf.write(slice).unwrap()
                            } else {
                                0
                            };
                            if bytes == 0 {
                                break 'outer;
                            }
                            bytes_written += bytes;
                        }
                    }

                    let pitch = pitch / 2;
                    let h = h / 2;
                    let w = w / 2;
                    let total_uv_size = w * h;

                    // U
                    let u_start = (self.read_offset + bytes_written - total_y_size) / w;
                    for i in u_start..h {
                        let offset = i * pitch;
                        let ptr = unsafe { data.__bindgen_anon_4.U.offset(offset as isize) };
                        debug_assert!(!ptr.is_null());
                        let slice: &[u8] = unsafe { std::slice::from_raw_parts(ptr, w) };
                        #[cfg(feature = "vector-write")]
                        {
                            let io_slice = std::io::IoSlice::new(slice);
                            io_slices.push(io_slice);
                        }
                        #[cfg(not(feature = "vector-write"))]
                        {
                            // We don't want to write a portion of a slice, only whole slices
                            let bytes = if slice.len() <= buf.len() {
                                // FIXME: remove unwrap
                                buf.write(slice).unwrap()
                            } else {
                                0
                            };
                            if bytes == 0 {
                                break 'outer;
                            }
                            bytes_written += bytes;
                        }
                    }

                    // V
                    let v_start =
                        (self.read_offset + bytes_written - total_y_size - total_uv_size) / w;
                    for i in v_start..h {
                        let offset = i * pitch;
                        let ptr = unsafe { data.__bindgen_anon_5.V.offset(offset as isize) };
                        debug_assert!(!ptr.is_null());
                        let slice: &[u8] = unsafe { std::slice::from_raw_parts(ptr, w) };
                        #[cfg(feature = "vector-write")]
                        {
                            let io_slice = std::io::IoSlice::new(slice);
                            io_slices.push(io_slice);
                        }
                        #[cfg(not(feature = "vector-write"))]
                        {
                            // We don't want to write a portion of a slice, only whole slices
                            let bytes = if slice.len() <= buf.len() {
                                // FIXME: remove unwrap
                                buf.write(slice).unwrap()
                            } else {
                                0
                            };
                            if bytes == 0 {
                                break 'outer;
                            }
                            bytes_written += bytes;
                        }
                    }
                    #[cfg(feature = "vector-write")]
                    {
                        bytes_written += io::Write::write_vectored(&mut buf, &io_slices).unwrap();
                    }
                    // dbg!(io_slices.len(), bytes_written);
                    // assert_eq!(buffers_written, (h as usize) * 2);
                }
                FourCC::NV12 => {
                    let pitch = unsafe { data.__bindgen_anon_2.Pitch } as usize;

                    // Y
                    let y_start = self.read_offset / w;
                    let total_y_size = w * h;
                    // dbg!(pitch, w, y_start, h, self.read_offset);
                    for i in y_start..h {
                        let offset = i * pitch;
                        let ptr = unsafe { data.__bindgen_anon_3.Y.offset(offset as isize) };
                        debug_assert!(!ptr.is_null());
                        // dbg!(i, offset, ptr, h, w, y_start);
                        let slice: &[u8] = unsafe { std::slice::from_raw_parts(ptr, w) };

                        // We don't want to write a portion of a slice, only whole slices
                        let bytes = if slice.len() <= buf.len() {
                            // FIXME: remove unwrap
                            buf.write(slice).unwrap()
                        } else {
                            0
                        };
                        if bytes == 0 {
                            break 'outer;
                        }
                        bytes_written += bytes;
                    }

                    let h = h / 2;

                    // U
                    let u_start = (self.read_offset + bytes_written - total_y_size) / w;
                    for i in u_start..h {
                        let offset = i * pitch;
                        let ptr = unsafe { data.__bindgen_anon_4.UV.offset(offset as isize) };
                        debug_assert!(!ptr.is_null());
                        let slice: &[u8] = unsafe { std::slice::from_raw_parts(ptr, w) };
                        // We don't want to write a portion of a slice, only whole slices
                        let bytes = if slice.len() <= buf.len() {
                            // FIXME: remove unwrap
                            buf.write(slice).unwrap()
                        } else {
                            0
                        };
                        if bytes == 0 {
                            break 'outer;
                        }
                        bytes_written += bytes;
                    }
                }
                // case MFX_FOURCC_NV12:
                //     // Y
                //     pitch = data->Pitch;
                //     for (i = 0; i < h; i++) {
                //         fwrite(data->Y + i * pitch, 1, w, f);
                //     }
                //     // UV
                //     h /= 2;
                //     for (i = 0; i < h; i++) {
                //         fwrite(data->UV + i * pitch, 1, w, f);
                //     }
                //     break;
                // case MFX_FOURCC_RGB4:
                //     // Y
                //     pitch = data->Pitch;
                //     for (i = 0; i < h; i++) {
                //         fwrite(data->B + i * pitch, 1, pitch, f);
                //     }
                //     break;
                _ => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Unsupported,
                        format!("Unsupported format {}", info.FourCC),
                    ));
                }
            };
        };

        // FIXME: Remove unwrap and replace with actual error
        self.unmap().unwrap();

        self.read_offset += bytes_written;
        Ok(bytes_written)
    }
}

pub struct Decoder<'a> {
    session: &'a mut Session,
}

impl<'a> Decoder<'a> {
    pub(crate) fn new(
        session: &'a mut Session,
        params: &mut MFXVideoParams,
    ) -> Result<Self, MfxStatus> {
        let lib = get_library().unwrap();

        let status: MfxStatus =
            unsafe { lib.MFXVideoDECODE_Init(session.inner, &mut params.inner) }.into();

        trace!("Decode init = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        let decoder = Self { session };

        Ok(decoder)
    }

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
        let lib = get_library().unwrap();

        // If bitstream is null than we are draining
        let bitstream = if let Some(bitstream) = bitstream {
            &mut bitstream.inner
        } else {
            std::ptr::null_mut()
        };

        let mut sync_point: ffi::mfxSyncPoint = std::ptr::null_mut();

        let mut output_surface: *mut ffi::mfxFrameSurface1 = std::ptr::null_mut();
        // dbg!(sync_point, output_surface);

        let status: MfxStatus = unsafe {
            lib.MFXVideoDECODE_DecodeFrameAsync(
                self.session.inner,
                // (isDraining) ? NULL : &bitstream,
                bitstream,
                std::ptr::null_mut(),
                &mut output_surface,
                &mut sync_point,
            )
        }
        .into();

        // dbg!(sync_point, output_surface);

        trace!("Decode frame start = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        let mut output_surface = FrameSurface::try_from(output_surface)?;

        output_surface.synchronize(timeout)?;


        let output_surface = task::spawn_blocking(move || {
            output_surface.synchronize(timeout)?;
            Ok(output_surface) as Result<FrameSurface, MfxStatus>
        })
        .await
        .unwrap()?;

        trace!("Decoded: {:?}", decode_start.elapsed());

        trace!("Decoded frame = {:?}", status);

        Ok(output_surface)
    }

    /// The application may use this API function to increase decoding performance by sacrificing output quality.
    ///
    /// See https://spec.oneapi.io/versions/latest/elements/oneVPL/source/API_ref/VPL_func_vid_decode.html#mfxvideodecode-setskipmode for more info.
    pub fn set_skip(&mut self, mode: SkipMode) -> Result<(), MfxStatus> {
        let lib = get_library().unwrap();

        let status: MfxStatus =
            unsafe { lib.MFXVideoDECODE_SetSkipMode(self.session.inner, mode.repr()) }.into();

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
    pub fn reset(&mut self, params: &mut MFXVideoParams) -> Result<(), MfxStatus> {
        let lib = get_library().unwrap();

        let status: MfxStatus =
            unsafe { lib.MFXVideoDECODE_Reset(self.session.inner, &mut params.inner) }.into();

        trace!("Decode reset = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        Ok(())
    }

    /// Retrieves current working parameters.
    ///
    /// See https://spec.oneapi.io/versions/latest/elements/oneVPL/source/API_ref/VPL_func_vid_decode.html#mfxvideodecode-getvideoparam for more info.
    pub fn get_params(&mut self) -> Result<MFXVideoParams, MfxStatus> {
        let lib = get_library().unwrap();

        let mut params = MFXVideoParams::new();

        let status: MfxStatus =
            unsafe { lib.MFXVideoDECODE_GetVideoParam(self.session.inner, &mut params.inner) }
                .into();

        trace!("Decode get params = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        Ok(params)
    }
}

impl<'a> Drop for Decoder<'a> {
    fn drop(&mut self) {
        let lib = get_library().unwrap();
        unsafe { lib.MFXVideoDECODE_Close(self.session.inner) };
    }
}

#[derive(Debug)]
pub enum AcceleratorHandle {
    VAAPI((File, *mut c_void)),
}

impl AcceleratorHandle {
    #[cfg(target_os = "linux")]
    /// If None is provided for file, a file at `/dev/dri/renderD128` is used.
    // TODO: We really should search /dev/dri/renderD128 - /dev/dri/renderD200 if file is None
    pub fn vaapi_from_file(file: Option<File>) -> Result<Self, MfxStatus> {
        use std::os::fd::AsRawFd;
        let file = file.unwrap_or_else(|| {
            File::options()
                .read(true)
                .write(true)
                .open("/dev/dri/renderD128")
                .unwrap()
        });

        let display = unsafe { libva_sys::va_display_drm::vaGetDisplayDRM(file.as_raw_fd()) };

        if display.is_null() {
            return Err(MfxStatus::InvalidHandle);
        }

        let va_status = unsafe { libva_sys::va_display_drm::vaInitialize(display, &mut 0, &mut 0) };

        trace!("Initialized va display = {}", va_status);

        Ok(Self::VAAPI((file, display)))
    }
    pub fn handle(&self) -> &*mut c_void {
        match self {
            AcceleratorHandle::VAAPI((_, handle)) => &handle,
        }
    }
    pub fn mfx_type(&self) -> u32 {
        match self {
            AcceleratorHandle::VAAPI(_) => ffi::mfxHandleType_MFX_HANDLE_VA_DISPLAY,
        }
    }
}

impl Drop for AcceleratorHandle {
    fn drop(&mut self) {
        match self {
            AcceleratorHandle::VAAPI((_, va_display)) => {
                unsafe { libva_sys::va_display_drm::vaTerminate(*va_display) };
            }
        }
    }
}

pub struct Session {
    inner: mfxSession,
}

impl Session {
    #[tracing::instrument]
    pub(crate) fn new(loader: &mut Loader, index: mfxU32) -> Result<Self, MfxStatus> {
        let lib = get_library().unwrap();
        let mut session: mfxSession = unsafe { mem::zeroed() };
        let status: MfxStatus =
            unsafe { lib.MFXCreateSession(loader.inner, index, &mut session) }.into();

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        debug!("Created a new session");

        Ok(Self { inner: session })
    }

    pub fn decoder(&mut self, params: &mut MFXVideoParams) -> Result<Decoder<'_>, MfxStatus> {
        Decoder::new(self, params)
    }

    /// You probably want to set the io_pattern to `IoPattern::OUT_SYSTEM_MEMORY`
    pub fn decode_header(
        &self,
        bitstream: &mut Bitstream,
        io_pattern: IoPattern,
    ) -> Result<MFXVideoParams, MfxStatus> {
        let lib = get_library().unwrap();

        let mut params = MFXVideoParams::new();
        params.set_codec(bitstream.codec());
        params.set_io_pattern(io_pattern);

        let status: MfxStatus = unsafe {
            lib.MFXVideoDECODE_DecodeHeader(self.inner, &mut bitstream.inner, &mut params.inner)
        }
        .into();

        let format =
            FourCC::from_repr(unsafe { params.inner.__bindgen_anon_1.mfx.FrameInfo.FourCC })
                .unwrap();

        trace!("Decode header = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        trace!("Decode output format = {:?}", format);

        Ok(params)
    }

    pub fn implementation(&self) -> Result<Implementation, MfxStatus> {
        let lib = get_library().unwrap();

        let mut implementation = 0i32;

        let status: MfxStatus = unsafe { lib.MFXQueryIMPL(self.inner, &mut implementation) }.into();

        trace!("Session implementation = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        let implementation = Implementation::from_repr(implementation as u32).unwrap();

        Ok(implementation)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let lib = get_library().unwrap();
        unsafe { lib.MFXClose(self.inner) };
    }
}

pub fn get_library() -> Result<&'static ffi::vpl, libloading::Error> {
    if let Some(vpl) = LIBRARY.get() {
        return Ok(vpl);
    }

    let library_name = libloading::library_filename("vpl");
    let lib = unsafe { ffi::vpl::new(library_name) }?;

    // FIXME: Check for failure (unwrap/expect)
    LIBRARY.set(lib);

    debug!("Dynamic library loaded successfully");

    Ok(get_library().unwrap())
}

#[derive(Copy, Clone, Debug)]
pub struct MFXVideoParams {
    inner: ffi::mfxVideoParam,
}

impl MFXVideoParams {
    pub fn new() -> Self {
        Self {
            inner: unsafe { mem::zeroed() },
        }
    }
    pub fn codec(&self) -> Codec {
        Codec::from_repr(unsafe { self.inner.__bindgen_anon_1.mfx.CodecId }).unwrap()
    }
    pub fn set_codec(&mut self, codec: Codec) {
        self.inner.__bindgen_anon_1.mfx.CodecId = codec as u32;
    }

    pub fn set_io_pattern(&mut self, pattern: IoPattern) {
        self.inner.IOPattern = pattern.bits();
    }
    pub fn size(&self) -> &ffi::mfxFrameInfo__bindgen_ty_1__bindgen_ty_1 {
        unsafe {
            &self
                .inner
                .__bindgen_anon_1
                .mfx
                .FrameInfo
                .__bindgen_anon_1
                .__bindgen_anon_1
        }
    }
}

#[derive(Debug)]
pub struct Bitstream<'a> {
    buffer: &'a mut [u8],
    pub(crate) inner: mfxBitstream,
}

impl<'a> Bitstream<'a> {
    /// Creates a data source/destination for encoded/decoded/processed data
    #[tracing::instrument]
    pub fn with_codec(buffer: &'a mut [u8], codec: Codec) -> Self {
        let mut bitstream: mfxBitstream = unsafe { mem::zeroed() };
        bitstream.Data = buffer.as_mut_ptr();
        bitstream.MaxLength = buffer.len() as u32;
        bitstream.__bindgen_anon_1.__bindgen_anon_1.CodecId = codec as u32;
        Self {
            buffer,
            inner: bitstream,
        }
    }

    pub fn codec(&self) -> Codec {
        Codec::from_repr(unsafe { self.inner.__bindgen_anon_1.__bindgen_anon_1.CodecId }).unwrap()
    }

    /// The size of the backing buffer
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// The amount of data currently in the bitstream
    pub fn size(&self) -> u32 {
        self.inner.DataLength
    }

    pub fn set_flags(&mut self, flags: BitstreamDataFlags) {
        self.inner.DataFlag = flags.bits();
    }
}

impl io::Write for Bitstream<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let data_offset = self.inner.DataOffset as usize;
        let data_len = self.inner.DataLength as usize;

        let slice = &mut self.buffer;

        if data_len >= slice.len() {
            return Ok(0);
        }

        if data_offset > 0 {
            // Move all data after DataOffset to the beginning of Data
            let data_end = data_offset + data_len;
            slice.copy_within(data_offset..data_end, 0);
            self.inner.DataOffset = 0;
        }

        let free_buffer_len = slice.len() - data_len;
        let copy_len = usize::min(free_buffer_len, buf.len());
        slice[data_len..data_len + copy_len].copy_from_slice(&buf[..copy_len]);
        self.inner.DataLength += copy_len as u32;

        Ok(copy_len)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
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
    use crate::constants::{Codec, Implementation};

    use super::*;
    use tracing_test::traced_test;

    const DEFAULT_BUFFER_SIZE: usize = 1024 * 1024 * 2; // 2MB

    #[test]
    #[traced_test]
    fn create_session() {
        let mut loader = Loader::new().unwrap();

        let config = loader.new_config().unwrap();
        // Set software decoding
        config
            .set_filter_property(
                "mfxImplDescription.Impl",
                Implementation::Software.repr().into(),
                None,
            )
            .unwrap();

        let config = loader.new_config().unwrap();
        // Set decode HEVC
        config
            .set_filter_property(
                "mfxImplDescription.mfxDecoderDescription.decoder.CodecID",
                Codec::HEVC.repr().into(),
                None,
            )
            .unwrap();

        let config = loader.new_config().unwrap();
        // Set required API version to 2.2
        config
            .set_filter_property(
                "mfxImplDescription.ApiVersion.Version",
                ((2u32 << 16) + 2).into(),
                None,
            )
            .unwrap();

        let _session = loader.new_session(0).unwrap();

        // TODO
        // accelHandle = InitAcceleratorHandle(session);
        // let accel_handle = null_mut();
    }

    #[traced_test]
    #[tokio::test]
    async fn decode_hevc_file_frame() {
        // Open file to read from
        let file = std::fs::File::open("tests/frozen.hevc").unwrap();

        let mut loader = Loader::new().unwrap();

        let config = loader.new_config().unwrap();
        // Set software decoding
        config
            .set_filter_property(
                "mfxImplDescription.Impl",
                Implementation::Software.repr().into(),
                None,
            )
            .unwrap();

        let config = loader.new_config().unwrap();
        // Set decode HEVC
        config
            .set_filter_property(
                "mfxImplDescription.mfxDecoderDescription.decoder.CodecID",
                Codec::HEVC.repr().into(),
                None,
            )
            .unwrap();

        let config = loader.new_config().unwrap();
        // Set required API version to 2.2
        config
            .set_filter_property(
                "mfxImplDescription.ApiVersion.Version",
                ((2u32 << 16) + 2).into(),
                None,
            )
            .unwrap();

        let mut session = loader.new_session(0).unwrap();

        let mut buffer: Vec<u8> = vec![0; DEFAULT_BUFFER_SIZE];
        let mut bitstream = Bitstream::with_codec(&mut buffer, Codec::HEVC);
        let free_buffer_len = (bitstream.len() - bitstream.size() as usize) as u64;
        let bytes_read =
            io::copy(&mut io::Read::take(file, free_buffer_len), &mut bitstream).unwrap();
        assert_ne!(bytes_read, 0);

        let mut params = session
            .decode_header(&mut bitstream, IoPattern::OUT_SYSTEM_MEMORY)
            .unwrap();

        let decoder = session.decoder(&mut params).unwrap();

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
            .set_filter_property(
                "mfxImplDescription.Impl",
                Implementation::Software.repr().into(),
                None,
            )
            .unwrap();

        let config = loader.new_config().unwrap();
        // Set decode HEVC
        config
            .set_filter_property(
                "mfxImplDescription.mfxDecoderDescription.decoder.CodecID",
                Codec::HEVC.repr().into(),
                None,
            )
            .unwrap();

        let config = loader.new_config().unwrap();
        // Set required API version to 2.2
        config
            .set_filter_property(
                "mfxImplDescription.ApiVersion.Version",
                ((2u32 << 16) + 2).into(),
                None,
            )
            .unwrap();

        let mut session = loader.new_session(0).unwrap();

        let mut buffer: Vec<u8> = vec![0; DEFAULT_BUFFER_SIZE];
        let mut bitstream = Bitstream::with_codec(&mut buffer, Codec::HEVC);
        let free_buffer_len = (bitstream.len() - bitstream.size() as usize) as u64;
        let bytes_read = io::copy(
            &mut io::Read::take(&mut file, free_buffer_len),
            &mut bitstream,
        )
        .unwrap();
        assert_ne!(bytes_read, 0);

        let mut params = session
            .decode_header(&mut bitstream, IoPattern::OUT_SYSTEM_MEMORY)
            .unwrap();

        let decoder = session.decoder(&mut params).unwrap();

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
            .set_filter_property(
                "mfxImplDescription.Impl",
                Implementation::Software.repr().into(),
                None,
            )
            .unwrap();

        let config = loader.new_config().unwrap();
        // Set decode HEVC
        config
            .set_filter_property(
                "mfxImplDescription.mfxDecoderDescription.decoder.CodecID",
                Codec::HEVC.repr().into(),
                None,
            )
            .unwrap();

        let config = loader.new_config().unwrap();
        // Set required API version to 2.2
        config
            .set_filter_property(
                "mfxImplDescription.ApiVersion.Version",
                ((2u32 << 16) + 2).into(),
                None,
            )
            .unwrap();

        let mut session = loader.new_session(0).unwrap();

        let mut buffer: Vec<u8> = vec![0; DEFAULT_BUFFER_SIZE];
        let mut bitstream = Bitstream::with_codec(&mut buffer, Codec::HEVC);
        let free_buffer_len = (bitstream.len() - bitstream.size() as usize) as u64;
        let bytes_read =
            io::copy(&mut io::Read::take(file, free_buffer_len), &mut bitstream).unwrap();
        assert_ne!(bytes_read, 0);

        let mut params = session
            .decode_header(&mut bitstream, IoPattern::OUT_SYSTEM_MEMORY)
            .unwrap();

        let decoder = session.decoder(&mut params).unwrap();

        let _frame = decoder.decode(Some(&mut bitstream), None).await.unwrap();
    }
}

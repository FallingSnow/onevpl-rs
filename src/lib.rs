use std::ffi::c_void;
use std::fs::File;
use std::io::{ErrorKind, Read};
use std::{
    io::{self, Write},
    mem,
    ops::Deref,
};

use bitstream::Bitstream;
use constants::{ApiVersion, FourCC, Implementation, IoPattern};
use decode::Decoder;
use encode::Encoder;
use ffi::{
    mfxConfig, mfxLoader, mfxSession, mfxStructVersion, mfxStructVersion__bindgen_ty_1, mfxU32,
    mfxVariant, MfxStatus,
};
use intel_onevpl_sys as ffi;

use once_cell::sync::OnceCell;
use tracing::{debug, error, trace, warn};
pub use videoparams::MfxVideoParams;
use vpp::VideoProcessor;

use crate::constants::{ChromaFormat, MemoryFlag};

pub mod bitstream;
pub mod constants;
pub mod decode;
pub mod encode;
pub mod utils;
mod videoparams;
pub mod vpp;

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
        self.set_filter_property("mfxHandleType", handle.mfx_type(), None)?;
        self.set_filter_property("mfxHDL", *handle.handle(), None)?;

        self.accelerator = Some(handle);

        Ok(())
    }

    /// This is a shortcut for making a [`Config`] manually via [`Loader::new_config`].
    pub fn set_filter_property(
        &mut self,
        name: &str,
        value: impl Into<utils::FilterProperty>,
        version: Option<mfxStructVersion>,
    ) -> Result<(), MfxStatus> {
        let config = self.new_config()?;
        config.set_filter_property(name, value, version)
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

    pub fn set_filter_property(
        self,
        name: &str,
        value: impl Into<utils::FilterProperty>,
        version: Option<mfxStructVersion>,
    ) -> Result<(), MfxStatus> {
        let lib = get_library().unwrap();
        let version = version.unwrap_or(mfxStructVersion {
            __bindgen_anon_1: mfxStructVersion__bindgen_ty_1 { Minor: 0, Major: 0 },
        });

        let value = value.into();

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
            name, value, status
        );

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct FrameRate(u32, u32);

impl FrameRate {
    pub fn new(numerator: u32, denominator: u32) -> Self {
        Self(numerator, denominator)
    }
}

impl From<(u32, u32)> for FrameRate {
    fn from(value: (u32, u32)) -> Self {
        Self(value.0, value.1)
    }
}

// // FrameSurfaces can either be created by us or by the intel API. If this was created by the intel API, FrameSurface's inner will always be Ownership::Borrowed.
// #[derive(Debug)]
// pub enum Ownership<'a, T> {
//     Borrowed(&'a mut T),
//     Owned(T),
// }

// impl<T> Ownership<'_, T> {
//     pub fn as_mut(&mut self) -> &mut T {
//         match self {
//             Ownership::Borrowed(t) => t,
//             Ownership::Owned(t) => t,
//         }
//     }
//     pub fn as_ref(&self) -> &T {
//         match self {
//             Ownership::Borrowed(t) => t,
//             Ownership::Owned(t) => t,
//         }
//     }
// }

#[derive(Debug)]
pub struct FrameSurface<'a> {
    inner: &'a mut ffi::mfxFrameSurface1,
    read_offset: usize,
    // backing_buffer: &'b [u8]
}

unsafe impl Send for FrameSurface<'_> {}

impl<'a> FrameSurface<'a> {
    // pub fn new(
    //     buffer: &[u8],
    //     frame_rate: FrameRate,
    //     fourcc: FourCC,
    //     width: u16,
    //     height: u16,
    // ) -> Self {
    //     let mut inner: ffi::mfxFrameSurface1 = unsafe { mem::zeroed() };
    //     inner.Info.ChromaFormat = ChromaFormat::YUV420.repr() as u16;
    //     inner.Info.FourCC = fourcc.repr();
    //     inner.Info.FrameRateExtN = frame_rate.0;
    //     inner.Info.FrameRateExtD = frame_rate.1;
    //     inner.Info.PicStruct = PicStruct::Progressive.repr() as u16;
    //     inner.Info.__bindgen_anon_1.__bindgen_anon_1.CropW = width;
    //     inner.Info.__bindgen_anon_1.__bindgen_anon_1.CropH = height;
    //     inner.Info.__bindgen_anon_1.__bindgen_anon_1.Width = align16(width);
    //     inner.Info.__bindgen_anon_1.__bindgen_anon_1.Height = align16(height);
    //     Self {
    //         inner: Ownership::Owned(inner),
    //         read_offset: 0,
    //     }
    // }
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
    fn map<'b>(&'b mut self, access: MemoryFlag) -> Result<(), MfxStatus> {
        // Get memory mapping function
        let func = self.interface().Map.unwrap();

        // Map surface data to get read access to it
        let status: MfxStatus = unsafe { func(self.inner, access.bits()) }.into();

        trace!("Map framesurface = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        Ok(())
    }

    /// Invalidates pointers of surface->Info.Data and sets them to NULL. See [`ffi::mfxFrameSurfaceInterface::Unmap`] for more info.
    fn unmap(&mut self) -> Result<(), MfxStatus> {
        // Get memory mapping function
        let func = self.interface().Unmap.unwrap();

        // Unmap surface data
        let status: MfxStatus = unsafe { func(self.inner) }.into();

        trace!("Unmap framesurface = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        Ok(())
    }

    /// Decrements the internal reference counter of the surface. See [`ffi::mfxFrameSurfaceInterface::Release`] for more info.
    fn release(&mut self) -> Result<(), MfxStatus> {
        // Get memory mapping function
        let func = self.interface().Release.unwrap();

        // Release the frame
        let status: MfxStatus = unsafe { func(self.inner) }.into();

        trace!("Release framesurface = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        Ok(())
    }

    /// Tries to read exactly one frame from buffer
    pub fn read_one_frame(
        &mut self,
        reader: &mut impl Read,
        reader_format: FourCC,
    ) -> Result<(), MfxStatus> {
        self.map(MemoryFlag::WRITE).unwrap();

        let info = &self.inner.Info;
        let crop_h = unsafe { info.__bindgen_anon_1.__bindgen_anon_1.CropH } as usize;
        let h = unsafe { info.__bindgen_anon_1.__bindgen_anon_1.Height } as usize;
        let crop_w = unsafe { info.__bindgen_anon_1.__bindgen_anon_1.CropW } as usize;
        let w = unsafe { info.__bindgen_anon_1.__bindgen_anon_1.Width } as usize;
        
        let data = &mut self.inner.Data;
        // let pitch = unsafe { data.__bindgen_anon_2.Pitch } as usize;

        // let expected_size = w * h * 2;
        // if buffer.len() < expected_size {;
        //     trace!("Buffer length {} is less than {}.", buffer.len(), expected_size);
        //     return Err(MfxStatus::MoreData);
        // }


        // We cannot rely on `self.inner.Info.FourCC` because the frame surface is created by the encoder/decoder. And the enc/dec only creates formats it can specifically read, even if the read order is just swapped (eg. YV12 and IYUV).
        let fourcc = FourCC::from_repr(self.inner.Info.FourCC).unwrap();
        // let chromaformat = ChromaFormat::from_repr(self.inner.Info.ChromaFormat as u32).unwrap();
        let mut read_func = || {
            match (fourcc, reader_format) {
                (FourCC::YV12 | FourCC::IyuvOrI420, FourCC::YV12) => {
                    // Y plane
                    {
                        let len = w * h;
                        assert!(unsafe {!data.__bindgen_anon_3.Y.is_null()});
                        let y = unsafe { std::slice::from_raw_parts_mut(data.__bindgen_anon_3.Y, len) };
                        
                        reader.read_exact(y).map_err(|e| {
                            if e.kind() == ErrorKind::UnexpectedEof {
                                MfxStatus::MoreData
                            } else {
                                warn!("{:?}", e);
                                MfxStatus::Unknown
                            }
                        })?;
                    }
    
                    // V plane
                    {
                        let h = h / 2;
                        let w = w / 2;
                        let len = w * h;
                        assert!(unsafe {!data.__bindgen_anon_5.V.is_null()});
                        let v = unsafe { std::slice::from_raw_parts_mut(data.__bindgen_anon_5.V, len) };
                        
                        reader.read_exact(v).map_err(|e| {
                            if e.kind() == ErrorKind::UnexpectedEof {
                                MfxStatus::MoreData
                            } else {
                                warn!("{:?}", e);
                                MfxStatus::Unknown
                            }
                        })?;
                    }
    
                    // U plane
                    {
                        let h = h / 2;
                        let w = w / 2;
                        let len = w * h;
                        assert!(unsafe {!data.__bindgen_anon_4.U.is_null()});
                        let u = unsafe { std::slice::from_raw_parts_mut(data.__bindgen_anon_4.U, len) };
                        
                        reader.read_exact(u).map_err(|e| {
                            if e.kind() == ErrorKind::UnexpectedEof {
                                MfxStatus::MoreData
                            } else {
                                warn!("{:?}", e);
                                MfxStatus::Unknown
                            }
                        })?;
                    }
                }
                (FourCC::YV12 | FourCC::IyuvOrI420, FourCC::IyuvOrI420) => {
                    // Y plane
                    {
                        let len = w * h;
                        assert!(unsafe {!data.__bindgen_anon_3.Y.is_null()});
                        let y = unsafe { std::slice::from_raw_parts_mut(data.__bindgen_anon_3.Y, len) };
                        
                        reader.read_exact(y).map_err(|e| {
                            if e.kind() == ErrorKind::UnexpectedEof {
                                MfxStatus::MoreData
                            } else {
                                warn!("{:?}", e);
                                MfxStatus::Unknown
                            }
                        })?;
                    }
    
                    // U plane
                    {
                        let h = h / 2;
                        let w = w / 2;
                        let len = w * h;
                        assert!(unsafe {!data.__bindgen_anon_4.U.is_null()});
                        let u = unsafe { std::slice::from_raw_parts_mut(data.__bindgen_anon_4.U, len) };
                        
                        reader.read_exact(u).map_err(|e| {
                            if e.kind() == ErrorKind::UnexpectedEof {
                                MfxStatus::MoreData
                            } else {
                                warn!("{:?}", e);
                                MfxStatus::Unknown
                            }
                        })?;
                    }
    
                    // V plane
                    {
                        let h = h / 2;
                        let w = w / 2;
                        let len = w * h;
                        assert!(unsafe {!data.__bindgen_anon_5.V.is_null()});
                        let v = unsafe { std::slice::from_raw_parts_mut(data.__bindgen_anon_5.V, len) };
                        
                        reader.read_exact(v).map_err(|e| {
                            if e.kind() == ErrorKind::UnexpectedEof {
                                MfxStatus::MoreData
                            } else {
                                warn!("{:?}", e);
                                MfxStatus::Unknown
                            }
                        })?;
                    }
                }
                (FourCC::NV12, FourCC::YV12) => todo!(),
                _ => unimplemented!(),
            };

            Ok(())
        };

        let result: Result<(), MfxStatus> = read_func();

        self.unmap().unwrap();

        result?;

        Ok(())
    }

    pub fn pitch_high(&self) -> u16 {
        self.inner.Data.PitchHigh
    }
    pub fn set_pitch_high(&mut self, pitch: u16) {
        self.inner.Data.PitchHigh = pitch;
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
        if value.is_null() {
            return Err(MfxStatus::NullPtr);
        }
        let frame_surface = Self {
            inner: unsafe { value.as_mut().unwrap() },
            read_offset: 0,
            // backing_surface: None,
        };

        // If timestamp is 0 set it to unknown
        if frame_surface.inner.Data.TimeStamp == 0 {
            frame_surface.inner.Data.TimeStamp = ffi::MFX_TIMESTAMP_UNKNOWN as u64;
        }

        Ok(frame_surface)
    }
}

impl io::Read for FrameSurface<'_> {
    fn read(&mut self, mut buf: &mut [u8]) -> std::io::Result<usize> {
        // FIXME: Remove unwrap and replace with actual error
        self.map(MemoryFlag::READ).unwrap();

        let data: ffi::mfxFrameData = self.inner.Data;
        let info: ffi::mfxFrameInfo = self.inner.Info;

        let h = unsafe { info.__bindgen_anon_1.__bindgen_anon_1.CropH } as usize;
        let w = unsafe { info.__bindgen_anon_1.__bindgen_anon_1.CropW } as usize;

        let mut bytes_written = 0;

        // We wrap this in a closure so we can capture the result. No matter
        // what the result is, we are still able to unmap the surface.
        let mut write_func = || {
            'outer: {
                // FIXME: Remove unwrap and replace with actual error
                match FourCC::from_repr(info.FourCC).unwrap() {
                    FourCC::IyuvOrI420 | FourCC::YV12 => {
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
                            bytes_written +=
                                io::Write::write_vectored(&mut buf, &io_slices).unwrap();
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
                            format!("Unsupported format {:?}", FourCC::from_repr(info.FourCC)),
                        ));
                    }
                };
            };

            Ok(())
        };

        let result = write_func();

        // FIXME: Remove unwrap and replace with actual error
        self.unmap().unwrap();

        result?;

        self.read_offset += bytes_written;
        Ok(bytes_written)
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

        // FIXME: Can't get it to display the pointer
        // trace!("Got va DRM display = {:p}", display);

        if display.is_null() {
            return Err(MfxStatus::InvalidHandle);
        }

        let va_status = unsafe { libva_sys::va_display_drm::vaInitialize(display, &mut 0, &mut 0) };

        trace!("Initialized va display = {}", va_status);

        // FIXME: We really need to replace MfxStatus returned everywhere with a custom error enum
        if va_status != libva_sys::VA_STATUS_SUCCESS as i32 {
            error!(
                "Failed to intialize va display = vaInitialize = {}",
                va_status
            );
            return Err(MfxStatus::NotInitialized);
        }

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

#[derive(Debug)]
pub struct Session {
    inner: mfxSession,
    accelerator: Option<AcceleratorHandle>,
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

        let session = Self {
            inner: session,
            accelerator: None,
        };

        debug!("Created a new session");
        debug!("API version = {:?}", session.version().unwrap());
        debug!("Implementation = {:?}", session.implementation().unwrap());

        // FIXME: accelerator should be passed through from the loader if it was already set
        Ok(session)
    }

    // Get a new instances of a decoder tied to this session
    pub fn decoder<'a: 'b, 'b>(&'a self, params: MfxVideoParams) -> Result<Decoder<'b>, MfxStatus> {
        Decoder::new(self, params)
    }

    // Get a new instances of a encoder tied to this session
    pub fn encoder<'a: 'b, 'b>(&'a self, params: MfxVideoParams) -> Result<Encoder<'b>, MfxStatus> {
        Encoder::new(self, params)
    }

    // Get a new instances of a video processor tied to this session
    pub fn video_processor(
        &self,
        params: &mut crate::vpp::VppVideoParams,
    ) -> Result<VideoProcessor<'_>, MfxStatus> {
        VideoProcessor::new(self, params)
    }

    /// Parses the input bitstream and fills returns a [`MfxVideoParams`] structure with appropriate values, such as resolution and frame rate, for the Init API function.
    pub fn decode_header(
        &self,
        bitstream: &mut Bitstream,
        io_pattern: IoPattern,
    ) -> Result<MfxVideoParams, MfxStatus> {
        let lib = get_library().unwrap();

        let mut params = MfxVideoParams::default();
        params.set_codec(bitstream.codec());
        params.set_io_pattern(io_pattern);

        let status: MfxStatus = unsafe {
            lib.MFXVideoDECODE_DecodeHeader(self.inner, &mut bitstream.inner, &mut **params)
        }
        .into();

        trace!("Decode header = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        let frame_info = unsafe { (**params).__bindgen_anon_1.mfx.FrameInfo };
        let format = FourCC::from_repr(frame_info.FourCC).unwrap();
        let height = unsafe { frame_info.__bindgen_anon_1.__bindgen_anon_1.CropH };
        let width = unsafe { frame_info.__bindgen_anon_1.__bindgen_anon_1.CropW };
        let framerate_n = frame_info.FrameRateExtN;
        let framerate_d = frame_info.FrameRateExtD;
        let colorspace = ChromaFormat::from_repr(frame_info.ChromaFormat as u32).unwrap();

        trace!(
            "Header params = {:?} {:?} {}x{} @ {}fps",
            format,
            colorspace,
            width,
            height,
            framerate_n as f32 / framerate_d as f32
        );

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

        let implementation = Implementation::from_bits_truncate(implementation as u32);

        Ok(implementation)
    }

    pub fn version(&self) -> Result<ApiVersion, MfxStatus> {
        let lib = get_library().unwrap();

        let mut version: ffi::mfxVersion = unsafe { mem::zeroed() };

        let status = unsafe { lib.MFXQueryVersion(self.inner, &mut version) }.into();

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        let version = ApiVersion::from(unsafe { version.Version });

        Ok(version)
    }

    /// You should probably be setting the accelerator on the loader then creating a session.
    pub fn set_accelerator(&mut self, handle: AcceleratorHandle) -> Result<(), MfxStatus> {
        let lib = get_library().unwrap();
        let status =
            unsafe { lib.MFXVideoCORE_SetHandle(self.inner, handle.mfx_type(), *handle.handle()) }
                .into();

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        self.accelerator = Some(handle);

        Ok(())
    }

    /// Initiates execution of an asynchronous function not already started and returns the status code after the specified asynchronous operation completes. If wait is zero, the function returns immediately. `wait` is in milliseconds and defaults to 1000.
    pub fn sync(
        &self,
        sync_point: ffi::mfxSyncPoint,
        wait: Option<u32>,
    ) -> Result<MfxStatus, MfxStatus> {
        let lib = get_library().unwrap();
        let status =
            unsafe { lib.MFXVideoCORE_SyncOperation(self.inner, sync_point, wait.unwrap_or(1000)) }
                .into();

        match status {
            MfxStatus::NoneOrDone => Ok(status),
            MfxStatus::NonePartialOutput => Ok(status),
            status => Err(status),
        }
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

#[cfg(test)]
mod tests {
    use crate::constants::{ApiVersion, Codec, Implementation};

    use super::*;
    use tracing_test::traced_test;

    #[test]
    #[traced_test]
    fn create_session() {
        let mut loader = Loader::new().unwrap();

        let config = loader.new_config().unwrap();
        // Set software decoding
        config
            .set_filter_property("mfxImplDescription.Impl", Implementation::SOFTWARE, None)
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

        let _session = loader.new_session(0).unwrap();

        // TODO
        // accelHandle = InitAcceleratorHandle(session);
        // let accel_handle = null_mut();
    }
}

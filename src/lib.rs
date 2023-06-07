use std::ffi::c_void;
use std::fs::File;
use std::io::{Read};
use std::{
    io::{self, Write},
    mem,
    ops::Deref,
};

use bitstream::Bitstream;
use constants::{ApiVersion, FourCC, ImplementationType, IoPattern};
use decode::Decoder;
use encode::Encoder;
pub use ffi::MfxStatus;
use ffi::{
    mfxConfig, mfxLoader, mfxSession, mfxStructVersion, mfxStructVersion__bindgen_ty_1, mfxU32,
    mfxVariant,
};
use intel_onevpl_sys as ffi;

use once_cell::sync::OnceCell;
#[cfg(target_os = "linux")]
use tracing::error;
use tracing::{debug, trace, warn};
pub use videoparams::MfxVideoParams;
use vpp::VideoProcessor;

use crate::constants::{ChromaFormat, MemoryFlag};
use crate::utils::str_from_null_terminated_utf8_i8;

pub mod bitstream;
pub mod constants;
pub mod decode;
pub mod encode;
mod tests;
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

    // TODO: Finish, already works, just need to iterate over implementations and return them
    pub fn implementations(&mut self) -> Result<Vec<()>, MfxStatus> {
        use std::ptr::null_mut;
        let mut caps = null_mut();
        let format = constants::ImplementationCapabilitiesDeliverFormat::Description;
        let mut i = 0;
        let mut status = MfxStatus::NoneOrDone;
        let mut implementations = Vec::new();

        let lib = get_library().unwrap();

        while status == MfxStatus::NoneOrDone {
            status = unsafe { lib.MFXEnumImplementations(self.inner, i, format.repr(), &mut caps) }
                .into();

            if status == MfxStatus::NotFound {
                break;
            }
            if status != MfxStatus::NoneOrDone {
                return Err(status);
            }
            let raw_description = unsafe {
                mem::transmute::<*mut c_void, *const ffi::mfxImplDescription>(caps)
                    .as_ref()
                    .unwrap()
            };

            dbg!(
                unsafe { str_from_null_terminated_utf8_i8(&raw_description.ImplName) }.to_string()
            );
            dbg!(unsafe { str_from_null_terminated_utf8_i8(&raw_description.License) }.to_string());
            dbg!(
                unsafe { str_from_null_terminated_utf8_i8(&raw_description.Keywords) }.to_string()
            );
            i += 1;
        }

        return Ok(implementations);
    }

    pub fn use_hardware(&mut self, yes: bool) {
        let value = match yes {
            true => constants::ImplementationType::HARDWARE,
            false => constants::ImplementationType::SOFTWARE,
        };
        self.set_filter_property("mfxImplDescription.Impl", value, None)
            .unwrap();
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
pub struct FrameSurfaceBounds {
    pub pitch: u16,
    pub width: u16,
    pub height: u16,
    pub crop_x: u16,
    pub crop_y: u16,
    pub crop_width: u16,
    pub crop_height: u16,
}

#[derive(Debug)]
pub struct FrameSurface<'a> {
    inner: &'a mut ffi::mfxFrameSurface1,
    read_offset: usize,
    buffer: Vec<u8>,
    // backing_buffer: &'b [u8]
}

unsafe impl Send for FrameSurface<'_> {}

impl<'a> FrameSurface<'a> {
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
        let status: MfxStatus = unsafe { func(self.inner, access.bits() as u32) }.into();

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

    #[inline]
    pub fn fourcc(&self) -> FourCC {
        FourCC::from_repr(self.inner.Info.FourCC as ffi::_bindgen_ty_5).unwrap()
    }

    /// pitch = Number of bytes in a row (video width in bytes + padding)
    pub fn bounds(&self) -> FrameSurfaceBounds {
        let pitch = unsafe { self.inner.Data.__bindgen_anon_2.PitchLow };
        let width = unsafe { self.inner.Info.__bindgen_anon_1.__bindgen_anon_1.Width };
        let height = unsafe { self.inner.Info.__bindgen_anon_1.__bindgen_anon_1.Height };
        let crop_x = unsafe { self.inner.Info.__bindgen_anon_1.__bindgen_anon_1.CropX };
        let crop_y = unsafe { self.inner.Info.__bindgen_anon_1.__bindgen_anon_1.CropY };
        let crop_width = unsafe { self.inner.Info.__bindgen_anon_1.__bindgen_anon_1.CropW };
        let crop_height = unsafe { self.inner.Info.__bindgen_anon_1.__bindgen_anon_1.CropH };
        FrameSurfaceBounds {
            pitch,
            width,
            height,
            crop_x,
            crop_y,
            crop_width,
            crop_height,
        }
    }

    /// b(), g(), r(), and a() provide the buffer for the entire frame. So if you are reading a BGRA frame, you can read the entire frame into the slice returned by b().
    pub fn b<'c, 'd: 'c>(&'c mut self) -> &'d mut [u8] {
        assert!(unsafe { !self.inner.Data.__bindgen_anon_5.B.is_null() });

        let pitch = unsafe { self.inner.Data.__bindgen_anon_2.PitchLow };
        let crop_height = unsafe { self.inner.Info.__bindgen_anon_1.__bindgen_anon_1.CropH };

        let length = match self.fourcc() {
            FourCC::Rgb4OrBgra => crop_height as usize * pitch as usize,
            _ => unimplemented!(),
        };
        unsafe { std::slice::from_raw_parts_mut(self.inner.Data.__bindgen_anon_5.B, length) }
    }

    pub fn g<'c, 'd: 'c>(&'c mut self) -> &'d mut [u8] {
        assert!(unsafe { !self.inner.Data.__bindgen_anon_4.G.is_null() });

        let pitch = unsafe { self.inner.Data.__bindgen_anon_2.PitchLow };
        let crop_height = unsafe { self.inner.Info.__bindgen_anon_1.__bindgen_anon_1.CropH };

        let length = match self.fourcc() {
            FourCC::Rgb4OrBgra => crop_height as usize * pitch as usize - 1,
            _ => unimplemented!(),
        };
        unsafe { std::slice::from_raw_parts_mut(self.inner.Data.__bindgen_anon_4.G, length) }
    }

    pub fn r<'c, 'd: 'c>(&'c mut self) -> &'d mut [u8] {
        assert!(unsafe { !self.inner.Data.__bindgen_anon_3.R.is_null() });

        let pitch = unsafe { self.inner.Data.__bindgen_anon_2.PitchLow };
        let crop_height = unsafe { self.inner.Info.__bindgen_anon_1.__bindgen_anon_1.CropH };

        let length = match self.fourcc() {
            FourCC::Rgb4OrBgra => crop_height as usize * pitch as usize - 2,
            _ => unimplemented!(),
        };
        unsafe { std::slice::from_raw_parts_mut(self.inner.Data.__bindgen_anon_3.R, length) }
    }

    pub fn a<'c, 'd: 'c>(&'c mut self) -> &'d mut [u8] {
        assert!(!self.inner.Data.A.is_null());

        let pitch = unsafe { self.inner.Data.__bindgen_anon_2.PitchLow };
        let crop_height = unsafe { self.inner.Info.__bindgen_anon_1.__bindgen_anon_1.CropH };

        let length = match self.fourcc() {
            FourCC::Rgb4OrBgra => crop_height as usize * pitch as usize - 3,
            _ => unimplemented!(),
        };
        unsafe { std::slice::from_raw_parts_mut(self.inner.Data.A, length) }
    }

    /// Remember to take pitch into account when writing to
    pub fn y<'c, 'd: 'c>(&'c mut self) -> &'d mut [u8] {
        assert!(unsafe { !self.inner.Data.__bindgen_anon_3.Y.is_null() });

        let pitch = unsafe { self.inner.Data.__bindgen_anon_2.PitchLow };
        let crop_height = unsafe { self.inner.Info.__bindgen_anon_1.__bindgen_anon_1.CropH };

        let length = match self.fourcc() {
            FourCC::NV12 | FourCC::YV12 | FourCC::IyuvOrI420 => {
                crop_height as usize * pitch as usize
            }
            FourCC::NV16 => todo!(),
            FourCC::YUY2 => todo!(),
            FourCC::P8 => todo!(),
            FourCC::P8Texture => todo!(),
            FourCC::P010 => todo!(),
            FourCC::P016 => todo!(),
            FourCC::P210 => todo!(),
            FourCC::AYUV => todo!(),
            FourCC::AyuvRgb4 => todo!(),
            FourCC::UYVY => todo!(),
            FourCC::Y210 => todo!(),
            FourCC::Y410 => todo!(),
            FourCC::Y216 => todo!(),
            FourCC::Y416 => todo!(),
            FourCC::NV21 => todo!(),
            FourCC::I010 => todo!(),
            FourCC::I210 => todo!(),
            FourCC::I422 => todo!(),
            _ => unimplemented!(),
        };
        unsafe { std::slice::from_raw_parts_mut(self.inner.Data.__bindgen_anon_3.Y, length) }
    }

    pub fn u<'c, 'd: 'c>(&'c mut self) -> &'d mut [u8] {
        assert!(unsafe { !self.inner.Data.__bindgen_anon_4.U.is_null() });

        let pitch = unsafe { self.inner.Data.__bindgen_anon_2.PitchLow };
        let crop_height = unsafe { self.inner.Info.__bindgen_anon_1.__bindgen_anon_1.CropH };

        let length = match self.fourcc() {
            FourCC::NV12 | FourCC::YV12 | FourCC::IyuvOrI420 => {
                (crop_height / 2) as usize * (pitch / 2) as usize
            }
            FourCC::NV16 => todo!(),
            FourCC::YUY2 => todo!(),
            FourCC::P8 => todo!(),
            FourCC::P8Texture => todo!(),
            FourCC::P010 => todo!(),
            FourCC::P016 => todo!(),
            FourCC::P210 => todo!(),
            FourCC::AYUV => todo!(),
            FourCC::AyuvRgb4 => todo!(),
            FourCC::UYVY => todo!(),
            FourCC::Y210 => todo!(),
            FourCC::Y410 => todo!(),
            FourCC::Y216 => todo!(),
            FourCC::Y416 => todo!(),
            FourCC::NV21 => todo!(),
            FourCC::I010 => todo!(),
            FourCC::I210 => todo!(),
            FourCC::I422 => todo!(),
            _ => unimplemented!(),
        };
        unsafe { std::slice::from_raw_parts_mut(self.inner.Data.__bindgen_anon_4.U, length) }
    }

    pub fn v<'c, 'd: 'c>(&'c mut self) -> &'d mut [u8] {
        assert!(unsafe { !self.inner.Data.__bindgen_anon_5.V.is_null() });

        let pitch = unsafe { self.inner.Data.__bindgen_anon_2.PitchLow };
        let crop_height = unsafe { self.inner.Info.__bindgen_anon_1.__bindgen_anon_1.CropH };

        let length = match self.fourcc() {
            FourCC::NV12 | FourCC::YV12 | FourCC::IyuvOrI420 => {
                (crop_height / 2) as usize * (pitch / 2) as usize
            }
            FourCC::NV16 => todo!(),
            FourCC::YUY2 => todo!(),
            FourCC::P8 => todo!(),
            FourCC::P8Texture => todo!(),
            FourCC::P010 => todo!(),
            FourCC::P016 => todo!(),
            FourCC::P210 => todo!(),
            FourCC::AYUV => todo!(),
            FourCC::AyuvRgb4 => todo!(),
            FourCC::UYVY => todo!(),
            FourCC::Y210 => todo!(),
            FourCC::Y410 => todo!(),
            FourCC::Y216 => todo!(),
            FourCC::Y416 => todo!(),
            FourCC::NV21 => todo!(),
            FourCC::I010 => todo!(),
            FourCC::I210 => todo!(),
            FourCC::I422 => todo!(),
            _ => unimplemented!(),
        };
        unsafe { std::slice::from_raw_parts_mut(self.inner.Data.__bindgen_anon_5.V, length) }
    }

    fn read_iyuv_or_i420_frame(&mut self) -> Result<(), MfxStatus> {
        let bounds = self.bounds();
        let crop_h = bounds.crop_height as usize;
        let crop_w = bounds.crop_width as usize;
        let pitch = bounds.pitch as usize;
        let mut read_offset = 0;

        let y = self.y();
        let u = self.u();
        let v = self.v();

        // Y plane
        {
            for i_h in 0..crop_h {
                let source_offset = i_h * crop_w;
                let offset = i_h * pitch;
                let source = &self.buffer[source_offset..source_offset + crop_w];
                let target = &mut y[offset..offset + crop_w];
                target.copy_from_slice(source);
            }
            read_offset += crop_h * crop_w;
        }

        // U plane
        {
            let pitch = pitch / 2;
            let crop_h = crop_h / 2;
            let crop_w = crop_w / 2;
            for i_h in 0..crop_h {
                let source_offset = read_offset + i_h * crop_w;
                let offset = i_h * pitch;
                let source = &self.buffer[source_offset..source_offset + crop_w];
                let target = &mut u[offset..offset + crop_w];
                target.copy_from_slice(source);
            }
            read_offset += crop_h * crop_w;
        }

        // V plane
        {
            let pitch = pitch / 2;
            let crop_h = crop_h / 2;
            let crop_w = crop_w / 2;
            for i_h in 0..crop_h {
                let source_offset = read_offset + i_h * crop_w;
                let offset = i_h * pitch;
                let source = &self.buffer[source_offset..source_offset + crop_w];
                let target = &mut v[offset..offset + crop_w];
                target.copy_from_slice(source);
            }
            // read_offset += crop_h * crop_w;
        }

        Ok(())
    }

    fn read_yv12_frame(&mut self) -> Result<(), MfxStatus> {
        let bounds = self.bounds();
        let crop_h = bounds.crop_height as usize;
        let crop_w = bounds.crop_width as usize;
        let pitch = bounds.pitch as usize;
        let mut read_offset = 0;

        let y = self.y();
        let u = self.u();
        let v = self.v();

        // Y plane
        {
            for i_h in 0..crop_h {
                let source_offset = i_h * crop_w;
                let offset = i_h * pitch;
                let source = &self.buffer[source_offset..source_offset + crop_w];
                let target = &mut y[offset..offset + crop_w];
                target.copy_from_slice(source);
            }
            read_offset += crop_h * crop_w;
        }

        // V plane
        {
            let pitch = pitch / 2;
            let crop_h = crop_h / 2;
            let crop_w = crop_w / 2;
            for i_h in 0..crop_h {
                let source_offset = read_offset + i_h * crop_w;
                let offset = i_h * pitch;
                let source = &self.buffer[source_offset..source_offset + crop_w];
                let target = &mut v[offset..offset + crop_w];
                target.copy_from_slice(source);
            }
            read_offset += crop_h * crop_w;
        }

        // U plane
        {
            let pitch = pitch / 2;
            let crop_h = crop_h / 2;
            let crop_w = crop_w / 2;
            for i_h in 0..crop_h {
                let source_offset = read_offset + i_h * crop_w;
                let offset = i_h * pitch;
                let source = &self.buffer[source_offset..source_offset + crop_w];
                let target = &mut u[offset..offset + crop_w];
                target.copy_from_slice(source);
            }
            // read_offset += crop_h * crop_w;
        }

        Ok(())
    }

    fn read_bgra_frame(&mut self) -> Result<(), MfxStatus> {
        let b = self.b();

        b.copy_from_slice(&self.buffer);

        Ok(())
    }

    /// Reads a single frame in the given pixel format. Unfortunately you need to pass the width and height of the frame because the frame's internal size is unreliable.
    pub fn read_raw_frame<R: Read>(
        &mut self,
        source: &mut R,
        format: FourCC,
    ) -> Result<(), MfxStatus> {
        self.map(MemoryFlag::WRITE).unwrap();

        match source.read_exact(&mut self.buffer) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(MfxStatus::MoreData);
            }
            Err(e) => {
                warn!("{}", e);
                return Err(MfxStatus::Unknown);
            }
        };

        let mut read_func = || match format {
            FourCC::NV12 => todo!(),
            FourCC::YV12 => self.read_yv12_frame(),
            FourCC::NV16 => todo!(),
            FourCC::YUY2 => todo!(),
            FourCC::RGB565 => todo!(),
            FourCC::RGBP => todo!(),
            FourCC::RGB3 => todo!(),
            FourCC::Rgb4OrBgra => self.read_bgra_frame(),
            FourCC::P8 => todo!(),
            FourCC::P8Texture => todo!(),
            FourCC::P010 => todo!(),
            FourCC::P016 => todo!(),
            FourCC::P210 => todo!(),
            FourCC::BGR4 => todo!(),
            FourCC::A2RGB10 => todo!(),
            FourCC::ARGB16 => todo!(),
            FourCC::ABGR16 => todo!(),
            FourCC::R16 => todo!(),
            FourCC::AYUV => todo!(),
            FourCC::AyuvRgb4 => todo!(),
            FourCC::UYVY => todo!(),
            FourCC::Y210 => todo!(),
            FourCC::Y410 => todo!(),
            FourCC::Y216 => todo!(),
            FourCC::Y416 => todo!(),
            FourCC::NV21 => todo!(),
            FourCC::IyuvOrI420 => self.read_iyuv_or_i420_frame(),
            FourCC::I010 => todo!(),
            FourCC::I210 => todo!(),
            FourCC::I422 => todo!(),
            FourCC::BGRP => todo!(),
        };

        let result: Result<(), MfxStatus> = read_func();

        self.unmap().unwrap();

        result
    }

    pub fn frame_size(format: FourCC, width: u16, height: u16) -> usize {
        match format {
            FourCC::IyuvOrI420 | FourCC::NV12 | FourCC::YV12 => {
                width as usize * height as usize * 3 / 2
            }
            FourCC::Rgb4OrBgra => width as usize * height as usize * 4,
            _ => todo!(),
        }
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

        let format =
            FourCC::from_repr(unsafe { (*value).Info.FourCC } as ffi::_bindgen_ty_5).unwrap();
        let width = unsafe { (*value).Info.__bindgen_anon_1.__bindgen_anon_1.CropW };
        let height = unsafe { (*value).Info.__bindgen_anon_1.__bindgen_anon_1.CropH };
        let frame_size = Self::frame_size(format, width, height);

        let frame_surface = Self {
            inner: unsafe { value.as_mut().unwrap() },
            read_offset: 0,
            buffer: vec![0u8; frame_size], // backing_surface: None,
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

        let bounds = self.bounds();
        let h = bounds.crop_height as usize;
        let w = bounds.crop_width as usize;
        let pitch = bounds.pitch as usize;

        let mut bytes_written = 0;

        // We wrap this in a closure so we can capture the result. No matter
        // what the result is, we are still able to unmap the surface.
        let mut write_func = || {
            'outer: {
                // FIXME: Remove unwrap and replace with actual error
                match FourCC::from_repr(info.FourCC as ffi::_bindgen_ty_5).unwrap() {
                    FourCC::IyuvOrI420 | FourCC::YV12 => {
                        #[cfg(feature = "vector-write")]
                        let mut io_slices: Vec<io::IoSlice> = Vec::with_capacity(h * 2);

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
                            format!(
                                "Unsupported format {:?}",
                                FourCC::from_repr(info.FourCC as ffi::_bindgen_ty_5)
                            ),
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
    pub fn mfx_type(&self) -> ffi::mfxHandleType {
        match self {
            AcceleratorHandle::VAAPI(_) => ffi::mfxHandleType_MFX_HANDLE_VA_DISPLAY,
        }
    }
}

impl Drop for AcceleratorHandle {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
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
        let format = FourCC::from_repr(frame_info.FourCC as ffi::_bindgen_ty_5).unwrap();
        let height = unsafe { frame_info.__bindgen_anon_1.__bindgen_anon_1.CropH };
        let width = unsafe { frame_info.__bindgen_anon_1.__bindgen_anon_1.CropW };
        let framerate_n = frame_info.FrameRateExtN;
        let framerate_d = frame_info.FrameRateExtD;
        let colorspace =
            ChromaFormat::from_repr(frame_info.ChromaFormat as ffi::_bindgen_ty_7).unwrap();

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

    pub fn implementation(&self) -> Result<ImplementationType, MfxStatus> {
        let lib = get_library().unwrap();

        let mut implementation = 0i32;

        let status: MfxStatus = unsafe { lib.MFXQueryIMPL(self.inner, &mut implementation) }.into();

        trace!("Session implementation = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        let implementation =
            ImplementationType::from_bits_truncate(implementation as ffi::mfxImplType);

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

    #[cfg(target_os = "windows")]
    let library_name = "libvpl";
    // let lib = unsafe { ffi::vpl::new(PathBuf::from("C:/Program Files (x86)/Intel/oneAPI/vpl/latest/bin/libvpl.dll")) }?;
    #[cfg(target_os = "linux")]
    let library_name = "vpl";
    let lib = {
        let library_name = libloading::library_filename(library_name);
        let lib = unsafe { ffi::vpl::new(library_name) }?;
        lib
    };

    // FIXME: Check for failure (unwrap/expect)
    LIBRARY.set(lib);

    debug!("Dynamic library loaded successfully");

    Ok(get_library().unwrap())
}

#[cfg(test)]
mod functional_tests {
    use crate::constants::{ApiVersion, Codec, ImplementationType};

    use super::*;
    use tracing_test::traced_test;

    #[test]
    #[traced_test]
    fn create_session() {
        let mut loader = Loader::new().unwrap();

        let config = loader.new_config().unwrap();
        // Set software decoding
        config
            .set_filter_property(
                "mfxImplDescription.Impl",
                ImplementationType::SOFTWARE,
                None,
            )
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

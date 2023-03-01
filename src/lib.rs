use std::{
    io::{self, Write},
    mem,
    ops::Deref,
    time::Instant,
};

use constants::{Codec, FourCC, IoPattern};
use ffi::{
    mfxBitstream, mfxConfig, mfxLoader, mfxSession, mfxStructVersion,
    mfxStructVersion__bindgen_ty_1, mfxU32, mfxVariant, mfxVariantType_MFX_VARIANT_TYPE_U32,
    mfxVariant_data, MfxStatus,
};
use intel_onevpl_sys as ffi;

use once_cell::sync::OnceCell;
use tracing::{debug, trace};

use crate::constants::MemoryFlag;

// use crate::callback_future::CbFuture;

// mod callback_future;
mod constants;

static LIBRARY: OnceCell<ffi::vpl> = OnceCell::new();

// The loader object remembers all created mfxConfig objects and destroys them during the mfxUnload function call.
#[derive(Debug)]
pub struct Loader {
    inner: mfxLoader,
    // configs: Configs
}
impl Loader {
    #[tracing::instrument]
    pub fn new() -> Result<Self, MfxStatus> {
        let lib = LIBRARY.get().unwrap();
        let loader = unsafe { lib.MFXLoad() };
        if loader.is_null() {
            return Err(MfxStatus::Unknown);
        }
        debug!("New loader created");

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

#[derive(Debug)]
pub struct Config {
    inner: mfxConfig,
}
impl Config {
    #[tracing::instrument]
    pub(crate) fn new(loader: &mut Loader) -> Result<Self, MfxStatus> {
        let lib = LIBRARY.get().unwrap();
        let config = unsafe { lib.MFXCreateConfig(loader.inner) };
        if config.is_null() {
            return Err(MfxStatus::Unknown);
        }
        return Ok(Self { inner: config });
    }

    #[tracing::instrument]
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

        let mut name = name.to_string();
        // CStrings need to nul terminated
        name.push('\0');

        let variant = mfxVariant {
            Version: version,
            Type: mfxVariantType_MFX_VARIANT_TYPE_U32,
            Data: mfxVariant_data { U32: value },
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

// struct DecodeFrameFuture {
//     sync_point: ffi::mfxSyncPoint,
//     output_surface: *mut ffi::mfxFrameSurface1,
// }

// impl Future for DecodeFrameFuture {
//     type Output = *mut ffi::mfxFrameSurface1;

//     fn poll(
//         self: std::pin::Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> std::task::Poll<Self::Output> {
//         let surface: ffi::mfxFrameSurface1 = unsafe { *self.output_surface };
//         let frame_interface: ffi::mfxFrameSurfaceInterface =
//             unsafe { *surface.__bindgen_anon_1.FrameInterface };
//         // let sync_func = frame_interface.OnComplete = ;
//         // sts = unsafe { sync_func(decSurfaceOut,
//         //             WAIT_100_MILLISECONDS) };
//         // if (MFX_ERR_NONE == sts) {
//         // sts = WriteRawFrame_InternalMem(decSurfaceOut, sink);
//         // VERIFY(MFX_ERR_NONE == sts, "Could not write decode output");

//         // framenum++;
//         // }

//         Poll::Pending
//     }
// }

pub struct FrameSurface<'a> {
    inner: &'a mut ffi::mfxFrameSurface1,
    read_offset: usize,
}

impl FrameSurface<'_> {
    /// Guarantees readiness of both the data (pixels) and any frame's meta information (for example corruption flags) after a function completes. See [`ffi::mfxFrameSurfaceInterface::Synchronize`] for more info.
    pub fn synchronize(&mut self) -> Result<(), MfxStatus> {
        let sync_func = self.interface().Synchronize.unwrap();
        let status: MfxStatus = unsafe { sync_func(self.inner, 100) }.into();

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
                    dbg!(u_start, h);
                    for i in u_start..h {
                        let offset = i * pitch;
                        let ptr = unsafe { data.__bindgen_anon_4.U.offset(offset as isize) };
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
        let lib = LIBRARY.get().unwrap();

        let status: MfxStatus =
            unsafe { lib.MFXVideoDECODE_Init(session.inner, &mut params.inner) }.into();

        trace!("Decode init = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        let decoder = Self { session };

        Ok(decoder)
    }

    pub async fn decode(
        &self,
        bitstream: Option<&mut Bitstream<'_>>,
    ) -> Result<FrameSurface, MfxStatus> {
        #[cfg(feature = "metrics")]
        let decode_start = Instant::now();
        let lib = LIBRARY.get().unwrap();

        // If bitstream is null than we are draining
        let bitstream = if let Some(bitstream) = bitstream {
            &mut bitstream.inner
        } else {
            std::ptr::null_mut()
        };

        let mut sync_point: ffi::mfxSyncPoint = std::ptr::null_mut();

        let mut output_surface: *mut ffi::mfxFrameSurface1 = std::ptr::null_mut();
        dbg!(sync_point, output_surface);

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

        dbg!(sync_point, output_surface);

        trace!("Decode frame start = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        let output_surface = FrameSurface::try_from(output_surface)?;

        // // This lets us set a callback on mfx structure and turn it into a future
        // let callback = {
        //     let cb: CbFuture<&mut ffi::mfxFrameSurface1> = CbFuture::new();
        //     // let func: unsafe extern "C" fn(sts: ffi::mfxStatus) = unsafe { transmute(|sts: i32| {
        //     //     println!("Done!")
        //     // })};

        //     cb.publish(result)

        //     fn func(status: i32) {
        //         println!("Decode done");
        //     }
        //     let func_ptr =
        //     func as fn(sts: i32);
        //     let func_ptr: unsafe extern "C" fn(sts: i32) = unsafe { std::mem::transmute(func_ptr) };

        //     let surface: ffi::mfxFrameSurface1 = unsafe { *output_surface };
        //     let mut frame_interface: ffi::mfxFrameSurfaceInterface =
        //         unsafe { *surface.__bindgen_anon_1.FrameInterface };
        //     frame_interface.OnComplete = Some(func_ptr);
        //     cb
        // };

        // Ok(callback.await)

        trace!("Decoded frame = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        #[cfg(feature = "metrics")]
        trace!("Decoded: {:?}", decode_start.elapsed());

        Ok(output_surface)
    }
}

impl<'a> Drop for Decoder<'a> {
    fn drop(&mut self) {
        let lib = LIBRARY.get().unwrap();
        unsafe { lib.MFXVideoDECODE_Close(self.session.inner) };
    }
}

pub struct Session {
    inner: mfxSession,
}
impl Session {
    #[tracing::instrument]
    pub(crate) fn new(loader: &mut Loader, index: mfxU32) -> Result<Self, MfxStatus> {
        let lib = LIBRARY.get().unwrap();
        let mut session: mfxSession = unsafe { mem::zeroed() };
        let status: MfxStatus =
            unsafe { lib.MFXCreateSession(loader.inner, index, &mut session) }.into();

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

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
        let lib = LIBRARY.get().unwrap();

        let mut params = MFXVideoParams::new();
        params.set_codec(bitstream.codec());
        params.set_io_pattern(io_pattern);

        let status: MfxStatus = unsafe {
            lib.MFXVideoDECODE_DecodeHeader(self.inner, &mut bitstream.inner, &mut params.inner)
        }
        .into();

        trace!("Decode header = {:?}", status);

        if status != MfxStatus::NoneOrDone {
            return Err(status);
        }

        Ok(params)
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
        todo!();
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

#[tracing::instrument]
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
    pub buffer: &'a mut [u8],
    pub(crate) inner: mfxBitstream,
}

impl<'a> Bitstream<'a> {
    /// Creates a data source/destination for encoded/decoded/processed data
    ///
    /// If source already contains data for the session to use, be sure to use `[Bitstream::set_len]` to set how many bytes of data the buffer contains
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

    pub fn set_len(&mut self, len: u32) {
        self.inner.DataLength = len;
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
    use std::io::Read;

    use crate::constants::{Impl, Codec};

    use super::*;
    use tracing_test::traced_test;

    const DEFAULT_BUFFER_SIZE: usize = 1024 * 200; // 200kB

    #[test]
    #[traced_test]
    fn create_session() {
        init().unwrap();
        let mut loader = Loader::new().unwrap();

        let config = loader.new_config().unwrap();
        // Set software decoding
        config
            .set_filter_property_u32(
                "mfxImplDescription.Impl",
                Impl::Software.repr(),
                None,
            )
            .unwrap();

        let config = loader.new_config().unwrap();
        // Set decode HEVC
        config
            .set_filter_property_u32(
                "mfxImplDescription.mfxDecoderDescription.decoder.CodecID",
                Codec::HEVC.repr(),
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

        let _session = loader.new_session(0).unwrap();

        // TODO
        // accelHandle = InitAcceleratorHandle(session);
        // let accel_handle = null_mut();
    }

    #[traced_test]
    #[tokio::test]
    async fn decode_hevc_file() {
        init().unwrap();

        // Open file to read from
        let mut file = std::fs::File::open("tests/frozen.hevc").unwrap();
        let mut output = std::fs::File::create("/tmp/output.yuv").unwrap();

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

        let mut session = loader.new_session(0).unwrap();

        let mut buffer: Vec<u8> = vec![0; DEFAULT_BUFFER_SIZE];
        let mut bitstream = Bitstream::with_codec(&mut buffer, Codec::HEVC);
        let bytes_read = file.read(bitstream.buffer).unwrap();
        bitstream.set_len(bytes_read as u32);

        let mut params = session
            .decode_header(&mut bitstream, IoPattern::OUT_SYSTEM_MEMORY)
            .unwrap();

        let decoder = session.decoder(&mut params).unwrap();

        {
            let mut frame = decoder.decode(Some(&mut bitstream)).await.unwrap();
            frame.synchronize().unwrap();
            frame.map().unwrap();
            let bytes = io::copy(&mut frame, &mut output).unwrap();
            dbg!(bytes);
        }
    }
}

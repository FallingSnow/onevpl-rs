use std::fmt::Debug;

use bitflags::bitflags;
use enum_repr::EnumRepr;
use intel_onevpl_sys as ffi;

use crate::utils::FilterProperty;

#[EnumRepr(type = "u32")]
#[derive(Debug, Clone, Copy)]
#[doc = " The ColorFourCC enumerator itemizes color formats."]
pub enum FourCC {
    #[doc = "< NV12 color planes. Native format for 4:2:0/8b Gen hardware implementation."]
    NV12 = ffi::MFX_FOURCC_NV12,
    #[doc = "< YV12 color planes."]
    YV12 = ffi::MFX_FOURCC_YV12,
    #[doc = "< 4:2:2 color format with similar to NV12 layout."]
    NV16 = ffi::MFX_FOURCC_NV16,
    #[doc = "< YUY2 color planes."]
    YUY2 = ffi::MFX_FOURCC_YUY2,
    #[doc = "< 2 bytes per pixel, uint16 in little-endian format, where 0-4 bits are blue, bits 5-10 are green and bits 11-15 are red."]
    RGB565 = ffi::MFX_FOURCC_RGB565,
    #[doc = " RGB 24 bit planar layout (3 separate channels, 8-bits per sample each). This format should be mapped to D3DFMT_R8G8B8 or VA_FOURCC_RGBP."]
    RGBP = ffi::MFX_FOURCC_RGBP,
    RGB3 = ffi::MFX_FOURCC_RGB3,
    #[doc = "< RGB4 (RGB32) color planes. BGRA is the order, 'B' is 8 MSBs, then 8 bits for 'G' channel, then 'R' and 'A' channels."]
    Rgb4OrBgra = ffi::MFX_FOURCC_RGB4,
    #[doc = "Internal color format. The application should use the following functions to create a surface that corresponds to the Direct3D* version in use.\n\nFor Direct3D* 9: IDirectXVideoDecoderService::CreateSurface()\n\nFor Direct3D* 11: ID3D11Device::CreateBuffer()"]
    P8 = ffi::MFX_FOURCC_P8,
    #[doc = "Internal color format. The application should use the following functions to create a surface that corresponds to the Direct3D* version in use.\n\nFor Direct3D 9: IDirectXVideoDecoderService::CreateSurface()\n\nFor Direct3D 11: ID3D11Device::CreateTexture2D()"]
    P8Texture = ffi::MFX_FOURCC_P8_TEXTURE,
    #[doc = "< P010 color format. This is 10 bit per sample format with similar to NV12 layout. This format should be mapped to DXGI_FORMAT_P010."]
    P010 = ffi::MFX_FOURCC_P010,
    #[doc = "< P016 color format. This is 16 bit per sample format with similar to NV12 layout. This format should be mapped to DXGI_FORMAT_P016."]
    P016 = ffi::MFX_FOURCC_P016,
    #[doc = "< 10 bit per sample 4:2:2 color format with similar to NV12 layout."]
    P210 = ffi::MFX_FOURCC_P210,
    #[doc = "< RGBA color format. It is similar to MFX_FOURCC_RGB4 but with different order of channels. 'R' is 8 MSBs, then 8 bits for 'G' channel, then 'B' and 'A' channels."]
    BGR4 = ffi::MFX_FOURCC_BGR4,
    #[doc = "< 10 bits ARGB color format packed in 32 bits. 'A' channel is two MSBs, then 'R', then 'G' and then 'B' channels. This format should be mapped to DXGI_FORMAT_R10G10B10A2_UNORM or D3DFMT_A2R10G10B10."]
    A2RGB10 = ffi::MFX_FOURCC_A2RGB10,
    #[doc = "< 10 bits ARGB color format packed in 64 bits. 'A' channel is 16 MSBs, then 'R', then 'G' and then 'B' channels. This format should be mapped to DXGI_FORMAT_R16G16B16A16_UINT or D3DFMT_A16B16G16R16 formats."]
    ARGB16 = ffi::MFX_FOURCC_ARGB16,
    #[doc = "< 10 bits ABGR color format packed in 64 bits. 'A' channel is 16 MSBs, then 'B', then 'G' and then 'R' channels. This format should be mapped to DXGI_FORMAT_R16G16B16A16_UINT or D3DFMT_A16B16G16R16 formats."]
    ABGR16 = ffi::MFX_FOURCC_ABGR16,
    #[doc = "< 16 bits single channel color format. This format should be mapped to DXGI_FORMAT_R16_TYPELESS or D3DFMT_R16F."]
    R16 = ffi::MFX_FOURCC_R16,
    #[doc = "< YUV 4:4:4, AYUV color format. This format should be mapped to DXGI_FORMAT_AYUV."]
    AYUV = ffi::MFX_FOURCC_AYUV,
    #[doc = "< RGB4 stored in AYUV surface. This format should be mapped to DXGI_FORMAT_AYUV."]
    AyuvRgb4 = ffi::MFX_FOURCC_AYUV_RGB4,
    #[doc = "< UYVY color planes. Same as YUY2 except the byte order is reversed."]
    UYVY = ffi::MFX_FOURCC_UYVY,
    #[doc = "< 10 bit per sample 4:2:2 packed color format with similar to YUY2 layout. This format should be mapped to DXGI_FORMAT_Y210."]
    Y210 = ffi::MFX_FOURCC_Y210,
    #[doc = "< 10 bit per sample 4:4:4 packed color format. This format should be mapped to DXGI_FORMAT_Y410."]
    Y410 = ffi::MFX_FOURCC_Y410,
    #[doc = "< 16 bit per sample 4:2:2 packed color format with similar to YUY2 layout. This format should be mapped to DXGI_FORMAT_Y216."]
    Y216 = ffi::MFX_FOURCC_Y216,
    #[doc = "< 16 bit per sample 4:4:4 packed color format. This format should be mapped to DXGI_FORMAT_Y416."]
    Y416 = ffi::MFX_FOURCC_Y416,
    #[doc = "< Same as NV12 but with weaved V and U values."]
    NV21 = ffi::MFX_FOURCC_NV21,
    #[doc = "< Same as  YV12 except that the U and V plane order is reversed."]
    IyuvOrI420 = ffi::MFX_FOURCC_IYUV,
    #[doc = "< 10-bit YUV 4:2:0, each component has its own plane."]
    I010 = ffi::MFX_FOURCC_I010,
    #[doc = "< 10-bit YUV 4:2:2, each component has its own plane."]
    I210 = ffi::MFX_FOURCC_I210,
    #[doc = "< Same as YV16 except that the U and V plane order is reversed"]
    I422 = ffi::MFX_FOURCC_I422,
    #[doc = " BGR 24 bit planar layout (3 separate channels, 8-bits per sample each). This format should be mapped to VA_FOURCC_BGRP."]
    BGRP = ffi::MFX_FOURCC_BGRP,
}

#[doc = " This enum itemizes hardware acceleration stack to use."]
#[EnumRepr(type = "u32")]
pub enum AccelerationMode {
    #[doc = "< Hardware acceleration is not applicable."]
    NA = ffi::mfxAccelerationMode_MFX_ACCEL_MODE_NA,
    #[doc = "< Hardware acceleration goes through the Microsoft* Direct3D9* infrastructure."]
    D3D9 = ffi::mfxAccelerationMode_MFX_ACCEL_MODE_VIA_D3D9,
    #[doc = "< Hardware acceleration goes through the Microsoft* Direct3D11* infrastructure."]
    D3D11 = ffi::mfxAccelerationMode_MFX_ACCEL_MODE_VIA_D3D11,
    #[doc = "< Hardware acceleration goes through the Linux* VA-API infrastructure or through the Linux* VA-API infrastructure with DRM RENDER MODE as default acceleration access point."]
    VAAPI = ffi::mfxAccelerationMode_MFX_ACCEL_MODE_VIA_VAAPI,
    #[doc = "< Hardware acceleration goes through the Linux* VA-API infrastructure with DRM MODESET as  default acceleration access point."]
    VAAPIDrmModeset = ffi::mfxAccelerationMode_MFX_ACCEL_MODE_VIA_VAAPI_DRM_MODESET,
    VAAPIGLX = ffi::mfxAccelerationMode_MFX_ACCEL_MODE_VIA_VAAPI_GLX,
    #[doc = "< Hardware acceleration goes through the Linux* VA-API infrastructure with X11 as default acceleration access point."]
    VAAPIX11 = ffi::mfxAccelerationMode_MFX_ACCEL_MODE_VIA_VAAPI_X11,
    #[doc = "< Hardware acceleration goes through the Linux* VA-API infrastructure with Wayland as default acceleration access point."]
    VAAPIWayland = ffi::mfxAccelerationMode_MFX_ACCEL_MODE_VIA_VAAPI_WAYLAND,
    #[doc = "< Hardware acceleration goes through the HDDL* Unite*."]
    HDDLUNITE = ffi::mfxAccelerationMode_MFX_ACCEL_MODE_VIA_HDDLUNITE,
}

bitflags! {
    #[doc = " The mfxMemoryFlags enumerator specifies memory access mode."]
    pub struct MemoryFlag: u32 {
        #[doc = "< The surface is mapped for reading."]
        const READ = ffi::mfxMemoryFlags_MFX_MAP_READ; // 1
        #[doc = "< The surface is mapped for writing."]
        const WRITE = ffi::mfxMemoryFlags_MFX_MAP_WRITE; // 2
        #[doc = " The mapping would be done immediately without any implicit synchronizations.\n \\attention This flag is optional."]
        const NO_WAIT = ffi::mfxMemoryFlags_MFX_MAP_NOWAIT; // 16
        #[doc = "< The surface is mapped for reading and writing."]
        const READ_WRITE = ffi::mfxMemoryFlags_MFX_MAP_READ_WRITE; // 3
    }
}

#[EnumRepr(type = "u32")]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

impl Into<FilterProperty> for Codec {
    fn into(self) -> FilterProperty {
        FilterProperty::U32(self.repr())
    }
}

pub struct ApiVersion(u32);

impl ApiVersion {
    pub fn new(major: u16, minor: u16) -> Self {
        ApiVersion(((major as u32) << 16) + minor as u32)
    }
    pub fn major(&self) -> u16 {
        (self.0 >> 16) as u16 
    }
    pub fn minor(&self) -> u16 {
        self.0 as u16 
    }
}

impl From<u32> for ApiVersion {
    fn from(value: u32) -> Self {
        ApiVersion(value)
    }
}

impl Into<FilterProperty> for ApiVersion {
    fn into(self) -> FilterProperty {
        FilterProperty::U32(self.0)
    }
}

impl Debug for ApiVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ApiVersion").field(&self.major()).field(&self.minor()).finish()
    }
}

// #[EnumRepr(type = "u32")]
// #[derive(Clone, Copy, PartialEq, Eq, Debug)]
// #[doc = " This enum itemizes implementation type."]
// pub enum Implementation {
//     #[doc = "< Pure Software Implementation."]
//     Software = ffi::mfxImplType_MFX_IMPL_TYPE_SOFTWARE,
//     #[doc = "< Hardware Accelerated Implementation."]
//     Hardware = ffi::mfxImplType_MFX_IMPL_TYPE_HARDWARE,
// }

bitflags! {
    #[doc = " This enum itemizes implementation type."]
    pub struct Implementation: u32 {
        #[doc = "< Pure Software Implementation."]
        const SOFTWARE = ffi::mfxImplType_MFX_IMPL_TYPE_SOFTWARE;
        #[doc = "< Hardware Accelerated Implementation."]
        const HARDWARE = ffi::mfxImplType_MFX_IMPL_TYPE_HARDWARE;
    }
}

impl Into<FilterProperty> for Implementation {
    fn into(self) -> FilterProperty {
        FilterProperty::U32(self.bits())
    }
}

bitflags! {
    #[doc = " The IOPattern enumerator itemizes memory access patterns for API functions. Use bit-ORed values to specify an input access\npattern and an output access pattern."]
    pub struct IoPattern: u16 {
        #[doc = "< Input to functions is a video memory surface."]
        const IN_VIDEO_MEMORY = ffi::MFX_IOPATTERN_IN_VIDEO_MEMORY as u16; // 1
        #[doc = "< Input to functions is a linear buffer directly in system memory or in system memory through an external allocator."]
        const IN_SYSTEM_MEMORY = ffi::MFX_IOPATTERN_IN_SYSTEM_MEMORY as u16; // 2
        #[doc = "< Output to functions is a video memory surface."]
        const OUT_VIDEO_MEMORY = ffi::MFX_IOPATTERN_OUT_VIDEO_MEMORY as u16; // 16
        #[doc = "< Output to functions is a linear buffer directly in system memory or in system memory through an external allocator."]
        const OUT_SYSTEM_MEMORY = ffi::MFX_IOPATTERN_OUT_SYSTEM_MEMORY as u16; // 32

        const SYSTEM_MEMORY = Self::IN_SYSTEM_MEMORY.bits | Self::OUT_SYSTEM_MEMORY.bits;
        const VIDEO_MEMORY = Self::IN_VIDEO_MEMORY.bits | Self::OUT_VIDEO_MEMORY.bits;
    }
}

bitflags! {
    #[doc = " The BitstreamDataFlag enumerator uses bit-ORed values to itemize additional information about the bitstream buffer."]
    pub struct BitstreamDataFlags: u16 {
        #[doc = "The bitstream buffer contains a complete frame or complementary field pair of data for the bitstream. For decoding, this means\nthat the decoder can proceed with this buffer without waiting for the start of the next frame, which effectively reduces decoding latency.\nIf this flag is set, but the bitstream buffer contains incomplete frame or pair of field, then decoder will produce corrupted output."]
        const COMPLETE_FRAME = ffi::MFX_BITSTREAM_COMPLETE_FRAME as u16;
        #[doc = "The bitstream buffer contains the end of the stream. For decoding,\nthis means that the application does not have any additional bitstream data to send to decoder."]
        const END_OF_STREAM = ffi::MFX_BITSTREAM_EOS as u16;
    }
}

#[doc = " The mfxSkipMode enumerator describes the decoder skip-mode options."]
#[EnumRepr(type = "u32")]
pub enum SkipMode {
    NoSkip = ffi::mfxSkipMode_MFX_SKIPMODE_NOSKIP,
    #[doc = " Do not skip any frames."]
    More = ffi::mfxSkipMode_MFX_SKIPMODE_MORE,
    #[doc = " Skip more frames."]
    Less = ffi::mfxSkipMode_MFX_SKIPMODE_LESS,
}

use std::fmt::Debug;

use bitflags::bitflags;
use enum_repr::EnumRepr;
use intel_onevpl_sys as ffi;

use crate::utils::FilterProperty;

#[cfg_attr(target_os = "unix", EnumRepr(type = "u32"))]
#[cfg_attr(target_os = "windows", EnumRepr(type = "i32"))]
#[derive(Debug, Clone, Copy)]
#[doc = " The SkipFrame enumerator is used to define usage of mfxEncodeCtrl::SkipFrame parameter."]
pub enum SkipFrame {
    #[doc = "< Frame skipping is disabled, mfxEncodeCtrl::SkipFrame is ignored."]
    NoSkip = ffi::MFX_SKIPFRAME_NO_SKIP,
    #[doc = "< Skipping is allowed, when mfxEncodeCtrl::SkipFrame is set encoder inserts into bitstream frame\nwhere all macroblocks are encoded as skipped. Only non-reference P- and B-frames can be skipped.\nIf GopRefDist = 1 and mfxEncodeCtrl::SkipFrame is set for reference P-frame, it will be encoded\nas non-reference."]
    InsertDummy = ffi::MFX_SKIPFRAME_INSERT_DUMMY,
    #[doc = "< Similar to MFX_SKIPFRAME_INSERT_DUMMY, but when mfxEncodeCtrl::SkipFrame is set encoder inserts nothing into bitstream."]
    InsertNothing = ffi::MFX_SKIPFRAME_INSERT_NOTHING,
    #[doc = "< mfxEncodeCtrl::SkipFrame indicates number of missed frames before the current frame. Affects only BRC, current frame will be encoded as usual."]
    BrcOnly = ffi::MFX_SKIPFRAME_BRC_ONLY,
}

#[cfg_attr(target_os = "unix", EnumRepr(type = "u32"))]
#[cfg_attr(target_os = "windows", EnumRepr(type = "i32"))]
#[derive(Debug, Clone, Copy)]
#[doc = " The FrameType enumerator itemizes frame types. Use bit-ORed values to specify all that apply."]
pub enum FrameType {
    #[doc = "< Frame type is unspecified."]
    Unknown = ffi::MFX_FRAMETYPE_UNKNOWN,
    #[doc = "< This frame or the first field is encoded as an I-frame/field."]
    I = ffi::MFX_FRAMETYPE_I,
    #[doc = "< This frame or the first field is encoded as an P-frame/field."]
    P = ffi::MFX_FRAMETYPE_P,
    #[doc = "< This frame or the first field is encoded as an B-frame/field."]
    B = ffi::MFX_FRAMETYPE_B,
    #[doc = "< This frame or the first field is either an SI- or SP-frame/field."]
    S = ffi::MFX_FRAMETYPE_S,
    #[doc = "< This frame or the first field is encoded as a reference."]
    Ref = ffi::MFX_FRAMETYPE_REF,
    #[doc = "< This frame or the first field is encoded as an IDR."]
    Idr = ffi::MFX_FRAMETYPE_IDR,
    #[doc = "< The second field is encoded as an I-field."]
    XI = ffi::MFX_FRAMETYPE_xI,
    #[doc = "< The second field is encoded as an P-field."]
    XP = ffi::MFX_FRAMETYPE_xP,
    #[doc = "< The second field is encoded as an S-field."]
    XB = ffi::MFX_FRAMETYPE_xB,
    #[doc = "< The second field is an SI- or SP-field."]
    XS = ffi::MFX_FRAMETYPE_xS,
    #[doc = "< The second field is encoded as a reference."]
    XRef = ffi::MFX_FRAMETYPE_xREF,
    #[doc = "< The second field is encoded as an IDR."]
    XIdr = ffi::MFX_FRAMETYPE_xIDR,
}

#[cfg_attr(target_os = "unix", EnumRepr(type = "u32"))]
#[cfg_attr(target_os = "windows", EnumRepr(type = "i32"))]
#[derive(Debug, Clone, Copy)]
#[doc = "The MfxNalUnitType enumerator specifies NAL unit types supported by the HEVC encoder."]
#[doc = "< See Table 7-1 of the ITU-T H.265 specification for the definition of these type."]
pub enum NalUnitType {
    #[doc = "< The encoder will decide what NAL unit type to use."]
    Unknown = ffi::MFX_HEVC_NALU_TYPE_UNKNOWN,
    TrailN = ffi::MFX_HEVC_NALU_TYPE_TRAIL_N,
    TrailR = ffi::MFX_HEVC_NALU_TYPE_TRAIL_R,
    RadlN = ffi::MFX_HEVC_NALU_TYPE_RADL_N,
    RadlR = ffi::MFX_HEVC_NALU_TYPE_RADL_R,
    RaslN = ffi::MFX_HEVC_NALU_TYPE_RASL_N,
    RaslR = ffi::MFX_HEVC_NALU_TYPE_RASL_R,
    IdrWRadl = ffi::MFX_HEVC_NALU_TYPE_IDR_W_RADL,
    IdrNLp = ffi::MFX_HEVC_NALU_TYPE_IDR_N_LP,
    CraNut = ffi::MFX_HEVC_NALU_TYPE_CRA_NUT,
}

#[cfg_attr(target_os = "unix", EnumRepr(type = "u32"))]
#[cfg_attr(target_os = "windows", EnumRepr(type = "i32"))]
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
    #[doc = "< Same as YV12 except that the U and V plane order is reversed."]
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
#[cfg_attr(target_os = "unix", EnumRepr(type = "u32"))]
#[cfg_attr(target_os = "windows", EnumRepr(type = "i32"))]
#[derive(Debug)]
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

#[derive(PartialEq, Eq, Clone, Copy)]
#[cfg_attr(target_os = "unix", EnumRepr(type = "u32"))]
#[cfg_attr(target_os = "windows", EnumRepr(type = "i32"))]
pub enum PicStruct {
    #[doc = "< Unspecified or mixed progressive/interlaced/field pictures."]
    Unknown = ffi::MFX_PICSTRUCT_UNKNOWN,
    #[doc = "< Progressive picture."]
    Progressive = ffi::MFX_PICSTRUCT_PROGRESSIVE,
    #[doc = "< Top field in first interlaced picture."]
    FieldTff = ffi::MFX_PICSTRUCT_FIELD_TFF,
    #[doc = "< Bottom field in first interlaced picture."]
    FieldBff = ffi::MFX_PICSTRUCT_FIELD_BFF,
    #[doc = "< First field repeated: pic_struct=5 or 6 in H.264."]
    FieldRepeated = ffi::MFX_PICSTRUCT_FIELD_REPEATED,
    #[doc = "< Double the frame for display: pic_struct=7 in H.264."]
    FrameDoubling = ffi::MFX_PICSTRUCT_FRAME_DOUBLING,
    #[doc = "< Triple the frame for display: pic_struct=8 in H.264."]
    FrameTripling = ffi::MFX_PICSTRUCT_FRAME_TRIPLING,
    #[doc = "< Single field in a picture."]
    FieldSingle = ffi::MFX_PICSTRUCT_FIELD_SINGLE,
    #[doc = "< Top field in a picture: pic_struct = 1 in H.265."]
    FieldTop = ffi::MFX_PICSTRUCT_FIELD_TOP,
    #[doc = "< Bottom field in a picture: pic_struct = 2 in H.265."]
    FieldBottom = ffi::MFX_PICSTRUCT_FIELD_BOTTOM,
    #[doc = "< Paired with previous field: pic_struct = 9 or 10 in H.265."]
    FieldPairedPrev = ffi::MFX_PICSTRUCT_FIELD_PAIRED_PREV,
    #[doc = "< Paired with next field: pic_struct = 11 or 12 in H.265"]
    FieldPairNext = ffi::MFX_PICSTRUCT_FIELD_PAIRED_NEXT,
}

bitflags! {
    #[doc = " The mfxMemoryFlags enumerator specifies memory access mode."]
    pub struct MemoryFlag: ffi::mfxMemoryFlags {
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

#[cfg_attr(target_os = "unix", EnumRepr(type = "u32"))]
#[cfg_attr(target_os = "windows", EnumRepr(type = "i32"))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[doc = " The TargetUsage enumerator itemizes a range of numbers from MFX_TARGETUSAGE_1, best quality, to MFX_TARGETUSAGE_7, best speed.\nIt indicates trade-offs between quality and speed. The application can use any number in the range. The actual number of supported\ntarget usages depends on implementation. If specified target usage is not supported, the encoder will use the closest supported value."]
pub enum TargetUsage {
    #[doc = "< Best quality"]
    Level1 = ffi::MFX_TARGETUSAGE_1,
    Level2 = ffi::MFX_TARGETUSAGE_2,
    Level3 = ffi::MFX_TARGETUSAGE_3,
    #[doc = "< Balanced quality and speed."]
    Level4 = ffi::MFX_TARGETUSAGE_4,
    Level5 = ffi::MFX_TARGETUSAGE_5,
    Level6 = ffi::MFX_TARGETUSAGE_6,
    #[doc = "< Best speed"]
    Level7 = ffi::MFX_TARGETUSAGE_7,
    #[doc = "< Unspecified target usage."]
    Unknown = ffi::MFX_TARGETUSAGE_UNKNOWN,
}

#[cfg_attr(target_os = "unix", EnumRepr(type = "u32"))]
#[cfg_attr(target_os = "windows", EnumRepr(type = "i32"))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[doc = " The RateControlMethod enumerator itemizes bitrate control methods."]
pub enum RateControlMethod {
    #[doc = "< Use the constant bitrate control algorithm."]
    CBR = ffi::MFX_RATECONTROL_CBR,
    #[doc = "< Use the variable bitrate control algorithm."]
    VBR = ffi::MFX_RATECONTROL_VBR,
    #[doc = "< Use the constant quantization parameter algorithm."]
    CQP = ffi::MFX_RATECONTROL_CQP,
    #[doc = "< Use the average variable bitrate control algorithm."]
    AVBR = ffi::MFX_RATECONTROL_AVBR,
    #[doc = "Use the VBR algorithm with look ahead. It is a special bitrate control mode in the AVC encoder that has been designed\nto improve encoding quality. It works by performing extensive analysis of several dozen frames before the actual encoding and as a side\neffect significantly increases encoding delay and memory consumption.\n\nThe only available rate control parameter in this mode is mfxInfoMFX::TargetKbps. Two other parameters, MaxKbps and InitialDelayInKB,\nare ignored. To control LA depth the application can use mfxExtCodingOption2::LookAheadDepth parameter.\n\nThis method is not HRD compliant."]
    LA = ffi::MFX_RATECONTROL_LA,
    #[doc = "Use the Intelligent Constant Quality algorithm. This algorithm improves subjective video quality of encoded stream. Depending on content,\nit may or may not decrease objective video quality. Only one control parameter is used - quality factor, specified by mfxInfoMFX::ICQQuality."]
    ICQ = ffi::MFX_RATECONTROL_ICQ,
    #[doc = "Use the Video Conferencing Mode algorithm. This algorithm is similar to the VBR and uses the same set of parameters mfxInfoMFX::InitialDelayInKB,\nTargetKbpsandMaxKbps. It is tuned for IPPP GOP pattern and streams with strong temporal correlation between frames.\nIt produces better objective and subjective video quality in these conditions than other bitrate control algorithms.\nIt does not support interlaced content, B-frames and produced stream is not HRD compliant."]
    VCM = ffi::MFX_RATECONTROL_VCM,
    #[doc = "Use Intelligent Constant Quality algorithm with look ahead. Quality factor is specified by mfxInfoMFX::ICQQuality.\nTo control LA depth the application can use mfxExtCodingOption2::LookAheadDepth parameter.\n\nThis method is not HRD compliant."]
    LAICQ = ffi::MFX_RATECONTROL_LA_ICQ,
    #[doc = " Use HRD compliant look ahead rate control algorithm."]
    LAHRD = ffi::MFX_RATECONTROL_LA_HRD,
    #[doc = "Use the variable bitrate control algorithm with constant quality. This algorithm trying to achieve the target subjective quality with\nthe minimum number of bits, while the bitrate constraint and HRD compliance are satisfied. It uses the same set of parameters\nas VBR and quality factor specified by mfxExtCodingOption3::QVBRQuality."]
    QVBR = ffi::MFX_RATECONTROL_QVBR,
}

#[cfg_attr(target_os = "unix", EnumRepr(type = "u32"))]
#[cfg_attr(target_os = "windows", EnumRepr(type = "i32"))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[doc = " The CodecFormatFourCC enumerator itemizes codecs in the FourCC format."]
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
        FilterProperty::U32(self.repr() as u32)
    }
}

#[cfg_attr(target_os = "unix", EnumRepr(type = "u32"))]
#[cfg_attr(target_os = "windows", EnumRepr(type = "i32"))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ImplementationCapabilitiesDeliverFormat {
    #[doc = "< Deliver capabilities as mfxImplDescription structure."]
    Description = ffi::mfxImplCapsDeliveryFormat_MFX_IMPLCAPS_IMPLDESCSTRUCTURE,
    #[doc = "< Deliver capabilities as mfxImplementedFunctions structure."]
    ImplementedFunctions = ffi::mfxImplCapsDeliveryFormat_MFX_IMPLCAPS_IMPLEMENTEDFUNCTIONS,
    #[doc = "< Deliver pointer to the null-terminated string with the path to the\nimplementation. String is delivered in a form of buffer of\nmfxChar type."]
    Path = ffi::mfxImplCapsDeliveryFormat_MFX_IMPLCAPS_IMPLPATH,
}

#[derive(Clone, Copy)]
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
        f.debug_tuple("ApiVersion")
            .field(&self.major())
            .field(&self.minor())
            .finish()
    }
}

#[doc = " This structure represents the implementation description."]
#[derive(Debug)]
pub struct Implementation {
    #[doc = "< Version of the structure."]
    pub version: ApiVersion,
    #[doc = "< Impl type: software/hardware."]
    pub implentation_type: ImplementationType,
    #[doc = "< Default Hardware acceleration stack to use. OS dependent parameter. Use VA for Linux* and DX* for Windows*."]
    pub acceleration_mode: AccelerationMode,
    #[doc = "< Supported API version."]
    pub api_verison: ApiVersion,
    #[doc = "< Null-terminated string with implementation name given by vendor."]
    pub implimentation_name: String,
    #[doc = "< Null-terminated string with comma-separated list of license names of the implementation."]
    pub license: String,
    #[doc = "< Null-terminated string with comma-separated list of keywords specific to this implementation that dispatcher can search for."]
    pub keywords: String,
    #[doc = "< Standard vendor ID 0x8086 - Intel."]
    pub vendor_id: ffi::mfxU32,
    #[doc = "< Vendor specific number with given implementation ID."]
    pub vendor_implementation_id: ffi::mfxU32,
    #[doc = "< Supported device."]
    pub dev: (), // TODO: mfxDeviceDescription,
    #[doc = "< Decoder configuration."]
    pub dec: (), // TODO: mfxDecoderDescription,
    #[doc = "< Encoder configuration."]
    pub enc: (), // TODO: mfxEncoderDescription,
    #[doc = "< VPP configuration."]
    pub vpp: (), // TODO: mfxVPPDescription,
    pub __bindgen_anon_1: (), // TODO: mfxImplDescription__bindgen_ty_1,
    #[doc = "< Supported surface pool polices."]
    pub pool_policies: (), // TODO: mfxPoolPolicyDescription,
    #[doc = "< Reserved for future use."]
    pub reserved: [ffi::mfxU32; 8usize],
    #[doc = "< Number of extension buffers. Reserved for future use. Must be 0."]
    pub num_ext_param: ffi::mfxU32,
    #[doc = "< Extension buffers. Reserved for future."]
    pub ext_params: (), // TODO: mfxImplDescription__bindgen_ty_2,
}

bitflags! {
    #[doc = " This enum itemizes implementation type."]
    pub struct ImplementationType: ffi::mfxImplType {
        #[doc = "< Pure Software Implementation."]
        const SOFTWARE = ffi::mfxImplType_MFX_IMPL_TYPE_SOFTWARE;
        #[doc = "< Hardware Accelerated Implementation."]
        const HARDWARE = ffi::mfxImplType_MFX_IMPL_TYPE_HARDWARE;
    }
}

impl Into<FilterProperty> for ImplementationType {
    fn into(self) -> FilterProperty {
        FilterProperty::U32(self.bits() as u32)
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
#[cfg_attr(target_os = "unix", EnumRepr(type = "u32"))]
#[cfg_attr(target_os = "windows", EnumRepr(type = "i32"))]
pub enum SkipMode {
    NoSkip = ffi::mfxSkipMode_MFX_SKIPMODE_NOSKIP,
    #[doc = " Do not skip any frames."]
    More = ffi::mfxSkipMode_MFX_SKIPMODE_MORE,
    #[doc = " Skip more frames."]
    Less = ffi::mfxSkipMode_MFX_SKIPMODE_LESS,
}

#[derive(Debug)]
#[cfg_attr(target_os = "unix", EnumRepr(type = "u32"))]
#[cfg_attr(target_os = "windows", EnumRepr(type = "i32"))]
pub enum ChromaFormat {
    #[doc = "< Monochrome or YUV400."]
    Monochrome = ffi::MFX_CHROMAFORMAT_MONOCHROME,
    #[doc = "< 4:2:0 color."]
    YUV420 = ffi::MFX_CHROMAFORMAT_YUV420,
    #[doc = "< 4:2:2 color with horizontal sub-sampling."]
    YUV422 = ffi::MFX_CHROMAFORMAT_YUV422,
    #[doc = "< 4:4:4 color."]
    YUV444 = ffi::MFX_CHROMAFORMAT_YUV444,
    #[doc = "< 4:1:1 color."]
    YUV411 = ffi::MFX_CHROMAFORMAT_YUV411,
    #[doc = "< 4:2:2 color with vertical sub-sampling."]
    YUV422V = ffi::MFX_CHROMAFORMAT_YUV422V,
    #[doc = "< Reserved."]
    Reserved1 = ffi::MFX_CHROMAFORMAT_RESERVED1,
}

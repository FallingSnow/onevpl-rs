use intel_onevpl_sys as ffi;

use crate::constants::PicStruct;

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

pub fn align16(x: u16) -> u16 {
    ((x + 15) >> 4) << 4
}

pub fn align32(x: u16) -> u16 {
    (x + 31) & !31
}

pub fn hw_align_width(width: u16) -> u16 {
    align16(width)
}
    // Needs to be multiple of 32 when picstruct is not progressive
pub fn hw_align_height(height: u16, picstruct: PicStruct) -> u16 {
    if picstruct == PicStruct::Progressive {
        align16(height)
    } else {
        align32(height)
    }
}
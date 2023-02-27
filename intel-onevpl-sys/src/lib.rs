#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::approx_constant)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::redundant_static_lifetimes)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::upper_case_acronyms)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(feature = "va")]
include!(concat!(env!("OUT_DIR"), "/bindings-va.rs"));

#[derive(Clone, Debug, Eq, PartialEq, Copy)]
#[repr(i32)]
pub enum MfxStatus {
    #[doc = "< No error or Task has been completed."]
    NoneOrDone = mfxStatus_MFX_ERR_NONE,
    #[doc = "< Unknown error."]
    Unknown = mfxStatus_MFX_ERR_UNKNOWN,
    #[doc = "< Null pointer."]
    NullPtr = mfxStatus_MFX_ERR_NULL_PTR,
    #[doc = "< Unsupported feature."]
    Unsupported = mfxStatus_MFX_ERR_UNSUPPORTED,
    #[doc = "< Failed to allocate memory."]
    MemoryAlloc = mfxStatus_MFX_ERR_MEMORY_ALLOC,
    #[doc = "< Insufficient buffer at input/output."]
    NotEnoughBuffer = mfxStatus_MFX_ERR_NOT_ENOUGH_BUFFER,
    #[doc = "< Invalid handle."]
    InvalidHandle = mfxStatus_MFX_ERR_INVALID_HANDLE,
    #[doc = "< Failed to lock the memory block."]
    LockMemory = mfxStatus_MFX_ERR_LOCK_MEMORY,
    #[doc = "< Member function called before initialization."]
    NotInitialized = mfxStatus_MFX_ERR_NOT_INITIALIZED,
    #[doc = "< The specified object is not found."]
    NotFound = mfxStatus_MFX_ERR_NOT_FOUND,
    #[doc = "< Expect more data at input."]
    MoreData = mfxStatus_MFX_ERR_MORE_DATA,
    #[doc = "< Expect more surface at output."]
    MoreSurface = mfxStatus_MFX_ERR_MORE_SURFACE,
    #[doc = "< Operation aborted."]
    Aborted = mfxStatus_MFX_ERR_ABORTED,
    #[doc = "< Lose the hardware acceleration device."]
    DeviceLost = mfxStatus_MFX_ERR_DEVICE_LOST,
    #[doc = "< Incompatible video parameters."]
    IncompatibleVideoParam = mfxStatus_MFX_ERR_INCOMPATIBLE_VIDEO_PARAM,
    #[doc = "< Invalid video parameters."]
    InvalidVideoParam = mfxStatus_MFX_ERR_INVALID_VIDEO_PARAM,
    #[doc = "< Undefined behavior."]
    UndefinedBehavior = mfxStatus_MFX_ERR_UNDEFINED_BEHAVIOR,
    #[doc = "< Device operation failure."]
    DeviceFailed = mfxStatus_MFX_ERR_DEVICE_FAILED,
    #[doc = "< Expect more bitstream buffers at output."]
    MoreBitstream = mfxStatus_MFX_ERR_MORE_BITSTREAM,
    #[doc = "< Device operation failure caused by GPU hang."]
    GpuHang = mfxStatus_MFX_ERR_GPU_HANG,
    #[doc = "< Bigger output surface required."]
    ReallocSurface = mfxStatus_MFX_ERR_REALLOC_SURFACE,
    #[doc = "< Write access is already acquired and user requested\nanother write access, or read access with MFX_MEMORY_NO_WAIT flag."]
    ResourceMapped = mfxStatus_MFX_ERR_RESOURCE_MAPPED,
    #[doc = "< Feature or function not implemented."]
    NotImplemented = mfxStatus_MFX_ERR_NOT_IMPLEMENTED,
    #[doc = "< The previous asynchronous operation is in execution."]
    InExecution = mfxStatus_MFX_WRN_IN_EXECUTION,
    #[doc = "< The hardware acceleration device is busy."]
    DeviceBusy = mfxStatus_MFX_WRN_DEVICE_BUSY,
    #[doc = "< The video parameters are changed during decoding."]
    VideoParamChanged = mfxStatus_MFX_WRN_VIDEO_PARAM_CHANGED,
    #[doc = "< Software acceleration is used."]
    PartialAcceleration = mfxStatus_MFX_WRN_PARTIAL_ACCELERATION,
    #[doc = "< Incompatible video parameters."]
    WarnIncompatibleVideoParam = mfxStatus_MFX_WRN_INCOMPATIBLE_VIDEO_PARAM,
    #[doc = "< The value is saturated based on its valid range."]
    ValueNotChanged = mfxStatus_MFX_WRN_VALUE_NOT_CHANGED,
    #[doc = "< The value is out of valid range."]
    OutOfRange = mfxStatus_MFX_WRN_OUT_OF_RANGE,
    #[doc = "< One of requested filters has been skipped."]
    FilterSkipped = mfxStatus_MFX_WRN_FILTER_SKIPPED,
    #[doc = "< Frame is not ready, but bitstream contains partial output."]
    NonePartialOutput = mfxStatus_MFX_ERR_NONE_PARTIAL_OUTPUT,
    #[doc = "< Timeout expired for internal frame allocation."]
    AllocTimeoutExpired = mfxStatus_MFX_WRN_ALLOC_TIMEOUT_EXPIRED,
    #[doc = "< There is some more work to do."]
    TaskWorking = mfxStatus_MFX_TASK_WORKING,
    #[doc = "< Task is waiting for resources."]
    TaskBusy = mfxStatus_MFX_TASK_BUSY,
    #[doc = "< Return MFX_ERR_MORE_DATA but submit internal asynchronous task."]
    MoreDataSubmitTask = mfxStatus_MFX_ERR_MORE_DATA_SUBMIT_TASK,
}

impl From<mfxStatus> for MfxStatus {
    fn from(v: i32) -> Self {
        match v {
            mfxStatus_MFX_ERR_MORE_DATA_SUBMIT_TASK => Self::MoreDataSubmitTask,
            // 11,-19 is not a valid status code
            v if v <= 13 && v >= -24 && v != 11 && v != -19 => unsafe { ::std::mem::transmute(v) },
            _ => Self::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mfx_status() {
        // mfxStatus_MFX_ERR_NONE and mfxStatus_MFX_TASK_DONE are both zero so only one can exist in enum
        assert_eq!(mfxStatus_MFX_ERR_NONE, mfxStatus_MFX_TASK_DONE);
    }
}

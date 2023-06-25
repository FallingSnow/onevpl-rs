use ffi::MfxStatus;
use intel_onevpl_sys as ffi;

pub struct FrameAllocator {
    inner: ffi::mfxFrameAllocator,
}

impl FrameAllocator {

}

pub type AllocFunc = dyn Fn(FrameAllocRequest) -> Result<FrameAllocResponse, MfxStatus>;
pub type LockFunc = dyn Fn(FrameAllocRequest) -> Result<FrameAllocResponse, MfxStatus>;

#[derive(Default)]
pub struct FrameAllocatorBuilder {
    alloc: Option<Box<AllocFunc>>,
    lock: Option<Box<LockFunc>>,
}

impl FrameAllocatorBuilder {
    pub fn set_alloc_callback(&mut self, callback: Box<AllocFunc>) -> &mut Self {
        self.alloc = Some(callback);
        self
    }
    pub fn set_lock_callback(&mut self, callback: Box<LockFunc>) -> &mut Self {
        self.lock = Some(callback);
        self
    }
}

#[doc = "Describes multiple frame allocations when initializing encoders, decoders, and video preprocessors.\nA range specifies the number of video frames. Applications are free to allocate additional frames. In all cases, the minimum number of\nframes must be at least NumFrameMin or the called API function will return an error."]
pub struct FrameAllocRequest {
    inner: ffi::mfxFrameAllocRequest
}

pub struct FrameAllocResponse {
    inner: ffi::mfxFrameAllocResponse
}

pub struct FrameData {
    inner: ffi::mfxFrameData
}
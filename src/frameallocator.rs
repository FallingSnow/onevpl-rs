use std::mem::zeroed;

use ffi::MfxStatus;
use intel_onevpl_sys as ffi;

use crate::constants::{Handle, MemId};

/// Allocates surface frames. For decoders, MFXVideoDECODE_Init calls Alloc only once. That call includes all frame allocation requests. For encoders, MFXVideoENCODE_Init calls Alloc twice: once for the input surfaces and again for the internal reconstructed surfaces.
///
/// If two library components must share DirectX* surfaces, this function should pass the pre-allocated surface chain to the library instead of allocating new DirectX surfaces.
///
/// See the Surface Pool Allocation section for additional information.
pub type Alloc = dyn Fn(&FrameAllocRequest) -> Result<FrameAllocResponse, MfxStatus>;
/// Locks a frame and returns its pointer.
pub type Lock = dyn Fn(MemId) -> Result<FrameData, MfxStatus>;
/// Unlocks a frame and invalidates the specified frame structure.
pub type Unlock = dyn Fn(MemId) -> Result<FrameData, MfxStatus>;
/// Unlocks a frame and invalidates the specified frame structure.
pub type GetHDL = dyn Fn(MemId) -> Result<Handle, MfxStatus>;
/// Unlocks a frame and invalidates the specified frame structure.
pub type Free = dyn Fn(&FrameAllocResponse) -> Result<FrameData, MfxStatus>;


pub struct FrameAllocator {
    alloc_callback: Option<Box<Alloc>>,
    lock_callback: Option<Box<Lock>>,
    unlock_callback: Option<Box<Unlock>>,
    get_hdl_callback: Option<Box<GetHDL>>,
    free_callback: Option<Box<Free>>,
    inner: ffi::mfxFrameAllocator,
}

impl FrameAllocator {
    pub fn new() -> Self {
        Self {
            alloc_callback: None,
            lock_callback: None,
            unlock_callback: None,
            get_hdl_callback: None,
            free_callback: None,
            inner: unsafe { zeroed() },
        }
    }
    // unsafe extern "C" fn alloc(pthis: *mut std::ffi::c_void, request: *mut ffi::mfxFrameAllocRequest, response: *mut ffi::mfxFrameAllocResponse) -> i32 {
    //     todo!()
    // }
    pub fn set_alloc_callback(&mut self, callback: Box<Alloc>) -> &mut Self {
        self.alloc_callback = Some(callback);
        self
    }
    pub fn set_lock_callback(&mut self, callback: Box<Lock>) -> &mut Self {
        self.lock_callback = Some(callback);
        self
    }
    pub fn set_unlock_callback(&mut self, callback: Box<Unlock>) -> &mut Self {
        self.unlock_callback = Some(callback);
        self
    }
    pub fn set_get_hdl_callback(&mut self, callback: Box<GetHDL>) -> &mut Self {
        self.get_hdl_callback = Some(callback);
        self
    }
    pub fn set_free_callback(&mut self, callback: Box<Free>) -> &mut Self {
        self.free_callback = Some(callback);
        self
    }
}

#[doc = "Describes multiple frame allocations when initializing encoders, decoders, and video preprocessors.\nA range specifies the number of video frames. Applications are free to allocate additional frames. In all cases, the minimum number of\nframes must be at least NumFrameMin or the called API function will return an error."]
pub struct FrameAllocRequest {
    inner: ffi::mfxFrameAllocRequest,
}

impl FrameAllocRequest {
    pub fn alloc_id(&self) -> u32 {
        unsafe { self.inner.__bindgen_anon_1.AllocId }
    }
}

#[doc = "Describes the response to multiple frame allocations. The calling API function returns the number of\nvideo frames actually allocated and pointers to their memory IDs."]
pub struct FrameAllocResponse {
    inner: ffi::mfxFrameAllocResponse,
}

impl FrameAllocResponse {
    /// The MemIds array is just an array of pointers. These pointers are basically IDs that are passed to the lock function to get the actual frame data, and passed to unlock invalidate a frame's data.
    ///
    /// - ids: Pointer to the array of the returned memory IDs. The application allocates or frees this array.
    /// - num_frames_actual: Number of frames actually allocated.
    pub fn new(request: &FrameAllocRequest, ids: &[MemId], num_frames_actual: u16) -> Self {
        let mut raw: ffi::mfxFrameAllocResponse = unsafe { zeroed() };
        let mut mids: Vec<*mut std::ffi::c_void> = ids.iter().map(|id| id.0).collect();
        raw.AllocId = request.alloc_id();
        raw.mids = mids.as_mut_ptr();
        raw.NumFrameActual = num_frames_actual;

        Self { inner: raw }
    }
}

#[doc = " Describes frame buffer pointers."]
pub struct FrameData {
    inner: ffi::mfxFrameData,
}

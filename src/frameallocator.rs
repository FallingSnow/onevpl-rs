use std::{ffi::c_void, fmt::Debug, mem::{zeroed, MaybeUninit}};

use ffi::{MfxStatus};
use intel_onevpl_sys as ffi;

use crate::{
    constants::{ExtMemFrameType, Handle, MemId},
    FrameInfo, FrameInfoMut,
};

/// Allocates surface frames. For decoders, MFXVideoDECODE_Init calls Alloc only once. That call includes all frame allocation requests. For encoders, MFXVideoENCODE_Init calls Alloc twice: once for the input surfaces and again for the internal reconstructed surfaces.
///
/// If two library components must share DirectX* surfaces, this function should pass the pre-allocated surface chain to the library instead of allocating new DirectX surfaces.
///
/// See the Surface Pool Allocation section for additional information.
pub type Alloc<'a> = dyn Fn(&FrameAllocRequest, &mut FrameAllocResponse) -> MfxStatus + 'a;
/// Locks a frame and returns its pointer.
pub type Lock<'a> = dyn Fn(MemId, &mut FrameDataMut) -> MfxStatus + 'a;
/// Unlocks a frame and invalidates the specified frame structure.
pub type Unlock<'a> = dyn Fn(MemId, &mut FrameDataMut) -> MfxStatus + 'a;
/// Unlocks a frame and invalidates the specified frame structure.
pub type GetHDL<'a> = dyn Fn(MemId, &mut MaybeUninit<Handle>) -> MfxStatus + 'a;
/// Unlocks a frame and invalidates the specified frame structure.
pub type Free<'a> = dyn Fn(&FrameAllocResponse) -> MfxStatus + 'a;

pub struct FrameAllocator<'a> {
    alloc_callback: Option<Box<Alloc<'a>>>,
    lock_callback: Option<Box<Lock<'a>>>,
    unlock_callback: Option<Box<Unlock<'a>>>,
    get_hdl_callback: Option<Box<GetHDL<'a>>>,
    free_callback: Option<Box<Free<'a>>>,
    pub(crate) inner: ffi::mfxFrameAllocator,
}

unsafe impl Send for FrameAllocator<'_> {}

impl Debug for FrameAllocator<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameAllocator")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<'a> FrameAllocator<'a> {
    pub fn new() -> Self {
        let inner: ffi::mfxFrameAllocator = unsafe { zeroed() };
        // FIXME: I think this pointer is invalidated as soon as we put inner in the struct

        let mut allocator = Self {
            alloc_callback: None,
            lock_callback: None,
            unlock_callback: None,
            get_hdl_callback: None,
            free_callback: None,
            inner,
        };

        allocator.inner.pthis = &mut allocator as *mut _ as *mut c_void;

        allocator
    }

    pub fn set_alloc_callback(&mut self, callback: Box<Alloc<'a>>) -> &mut Self {
        extern "C" fn alloc(
            pthis: *mut c_void,
            request: *mut ffi::mfxFrameAllocRequest,
            response: *mut ffi::mfxFrameAllocResponse,
        ) -> i32 {
            let allocator: &mut FrameAllocator = unsafe { std::mem::transmute(pthis) };
            let callback = match &allocator.alloc_callback {
                Some(c) => c,
                None => return MfxStatus::MemoryAlloc as i32,
            };

            let request = unsafe {
                FrameAllocRequest {
                    inner: request.as_mut().unwrap(),
                }
            };
            let mut response = unsafe {
                FrameAllocResponse {
                    inner: response.as_mut().unwrap(),
                }
            };

            callback(&request, &mut response) as i32
        }

        // Store the callback on the struct so it does not get destructed
        self.alloc_callback = Some(callback);

        // Assign the pointer to the C struct
        self.inner.Alloc = Some(alloc);

        self
    }

    pub fn set_lock_callback(&mut self, callback: Box<Lock<'a>>) -> &mut Self {
        extern "C" fn lock(
            pthis: *mut c_void,
            id: ffi::mfxMemId,
            data: *mut ffi::mfxFrameData,
        ) -> i32 {
            let allocator: &mut FrameAllocator = unsafe { std::mem::transmute(pthis) };
            let callback = match &allocator.lock_callback {
                Some(c) => c,
                None => return MfxStatus::MemoryAlloc as i32,
            };

            let id = MemId(id);
            let mut data = FrameDataMut {
                inner: unsafe {data.as_mut().unwrap()},
            };

            callback(id, &mut data) as i32
        }

        // Store the callback on the struct so it does not get destructed
        self.lock_callback = Some(callback);

        // Assign the pointer to the C struct
        self.inner.Lock = Some(lock);

        self
    }

    pub fn set_unlock_callback(&mut self, callback: Box<Unlock<'a>>) -> &mut Self {
        extern "C" fn unlock(
            pthis: *mut c_void,
            id: ffi::mfxMemId,
            data: *mut ffi::mfxFrameData,
        ) -> i32 {
            let allocator: &mut FrameAllocator = unsafe { std::mem::transmute(pthis) };
            let callback = match &allocator.unlock_callback {
                Some(c) => c,
                None => return MfxStatus::MemoryAlloc as i32,
            };

            let id = MemId(id);
            let mut data = FrameDataMut {
                inner: unsafe {data.as_mut().unwrap()},
            };

            callback(id, &mut data) as i32
        }

        // Store the callback on the struct so it does not get destructed
        self.unlock_callback = Some(callback);

        // Assign the pointer to the C struct
        self.inner.Unlock = Some(unlock);

        self
    }

    pub fn set_get_hdl_callback(&mut self, callback: Box<GetHDL<'a>>) -> &mut Self {
        extern "C" fn get_hdl(
            pthis: *mut c_void, mid: ffi::mfxMemId, _handle: *mut *mut c_void
        ) -> i32 {
            let allocator: &mut FrameAllocator = unsafe { std::mem::transmute(pthis) };
            let _callback = match &allocator.get_hdl_callback {
                Some(c) => c,
                None => return MfxStatus::MemoryAlloc as i32,
            };

            let _id = MemId(mid);
            
            todo!();
            // callback(id, handle as &mut _) as i32
        }

        // Store the callback on the struct so it does not get destructed
        self.get_hdl_callback = Some(callback);

        // Assign the pointer to the C struct
        self.inner.GetHDL = Some(get_hdl);

        self
    }

    pub fn set_free_callback(&mut self, callback: Box<Free<'a>>) -> &mut Self {
        extern "C" fn free(
            pthis: *mut c_void, response: *mut ffi::mfxFrameAllocResponse
        ) -> i32 {
            let allocator: &mut FrameAllocator = unsafe { std::mem::transmute(pthis) };
            let callback = match &allocator.free_callback {
                Some(c) => c,
                None => return MfxStatus::MemoryAlloc as i32,
            };

            let response = unsafe {
                FrameAllocResponse {
                    inner: response.as_mut().unwrap(),
                }
            };

            callback(&response) as i32
        }

        // Store the callback on the struct so it does not get destructed
        self.free_callback = Some(callback);

        // Assign the pointer to the C struct
        self.inner.Free = Some(free);

        self
    }
}

#[doc = "Describes multiple frame allocations when initializing encoders, decoders, and video preprocessors.\nA range specifies the number of video frames. Applications are free to allocate additional frames. In all cases, the minimum number of\nframes must be at least NumFrameMin or the called API function will return an error."]
pub struct FrameAllocRequest<'a> {
    inner: &'a mut ffi::mfxFrameAllocRequest,
}

impl FrameAllocRequest<'_> {
    pub fn alloc_id(&self) -> u32 {
        unsafe { self.inner.__bindgen_anon_1.AllocId }
    }
    pub fn num_frame_min(&self) -> u16 {
        self.inner.NumFrameMin
    }
    pub fn num_frame_suggested(&self) -> u16 {
        self.inner.NumFrameSuggested
    }
    pub fn info(&self) -> FrameInfo {
        FrameInfo {
            inner: &self.inner.Info,
        }
    }
    pub fn info_mut(&mut self) -> FrameInfoMut {
        FrameInfoMut {
            inner: &mut self.inner.Info,
        }
    }
    pub fn type_(&self) -> Option<ExtMemFrameType> {
        ExtMemFrameType::try_from(self.inner.Type as ffi::_bindgen_ty_36).ok()
    }
}

#[doc = "Describes the response to multiple frame allocations. The calling API function returns the number of\nvideo frames actually allocated and pointers to their memory IDs."]
pub struct FrameAllocResponse<'a> {
    inner: &'a mut ffi::mfxFrameAllocResponse,
}

impl FrameAllocResponse<'_> {
    /// The MemIds array is just an array of pointers. These pointers are basically IDs that are passed to the lock function to get the actual frame data, and passed to unlock invalidate a frame's data.
    pub fn set_mids(&mut self, mids: Vec<MemId>) {
        // Vector needs to be the same capacity as length because we need to destruct it later and we can't carry information about both capacity and length
        assert_eq!(mids.capacity(), mids.len(), "MemId Vector length != capacity");

        self.inner.NumFrameActual = mids.len().try_into().unwrap();
        let mut ptr: ffi::mfxMemId = mids.as_ptr().cast_mut() as *mut c_void;
        // We are now manually maintaining the lifetime of Vec<MemId>, this must be cleared when free is called
        std::mem::forget(mids);
        self.inner.mids = &mut ptr;

    }
}

#[doc = " Describes frame buffer pointers."]
pub struct FrameDataMut<'a> {
    inner: &'a mut ffi::mfxFrameData,
}

impl<'a> FrameDataMut<'a> {
    pub fn set_y(&mut self, target: &mut [u8]) {
        self.inner.__bindgen_anon_3.Y = target.as_mut_ptr();
    }
}

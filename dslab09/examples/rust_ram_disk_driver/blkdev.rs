use alloc::boxed::Box;
use core::ops::{DerefMut, Range};
use core::{marker, mem, ptr};

use linux_kernel_module::{bindings, c_types, CStr, Error, KernelResult};

use crate::more_bindings;
use core::ptr::null_mut;

static DISK_NAME_LEN: usize = 32;
pub static KERNEL_SECTOR_SIZE: usize = 512;

/* There is a lot of raw pointers in this file. They work similarly as pointers in C,
   but have to be wrapped in unsafe blocks of code when being dereferenced. */

pub fn builder(name: &'static CStr, minors: Range<u16>) -> KernelResult<Builder> {
    Ok(Builder { name, minors })
}

pub struct Builder {
    name: &'static CStr,
    minors: Range<u16>,
}

struct BlockDevice {
    gd: *mut bindings::gendisk,
    queue: *mut bindings::request_queue,
    lock: Box<bindings::spinlock_t>,
    device_description: Option<Box<dyn BlockDeviceDescription>>,
}

impl Drop for BlockDevice {
    fn drop(&mut self) {
        // This is OK, because of how Builder::build is implemented.
        // When gendisk (gd) is being set up, it can either be set up in full or not at all.
        // Queue is similar.
        unsafe {
            if !self.gd.is_null() {
                more_bindings::del_gendisk_wrapper(self.gd);
                more_bindings::put_disk_wrapper(self.gd);
                Box::from_raw((*self.gd).fops as *mut bindings::block_device_operations);
            }

            if !self.queue.is_null() {
                more_bindings::blk_cleanup_queue_wrapper(self.queue);
            }
        }
    }
}

pub struct BlockOperationsVtable(
    pub(crate) bindings::block_device_operations,
    pub(crate) unsafe extern "C" fn(*mut bindings::request_queue),
);

pub struct Request {
    result: c_types::c_uchar,
    pub(crate) request: *mut bindings::request,
}

impl Request {
    pub fn is_passthrough(&self) -> bool {
        // Simple FFI call.
        unsafe { more_bindings::blk_rq_is_passthrough_wrapper(self.request) != 0 }
    }

    pub fn set_error(&mut self, e: u8) {
        self.result = e;
    }
}

impl Drop for Request {
    fn drop(&mut self) {
        // Simple FFI call.
        unsafe {
            more_bindings::__blk_end_request_all_wrapper(self.request, self.result);
        }
    }
}

pub struct RequestQueue(*mut bindings::request_queue);

impl RequestQueue {
    pub fn fetch_request(&mut self) -> Option<Request> {
        // Simple FFI call.
        unsafe {
            let request = more_bindings::blk_fetch_request_wrapper(self.0);
            if !request.is_null() {
                Some(Request { result: 0, request })
            } else {
                None
            }
        }
    }
}

unsafe extern "C" fn open_callback<T: BlockDeviceDescription>(
    _bdev: *mut bindings::block_device,
    _mode: bindings::fmode_t,
) -> c_types::c_int {
    0
}

unsafe extern "C" fn release_callback<T: BlockDeviceDescription>(
    _gd: *mut bindings::gendisk,
    _mode: bindings::fmode_t,
) {
}

unsafe extern "C" fn transfer_callback<T: BlockDeviceDescription>(
    queue: *mut bindings::request_queue,
) {
    let mut rq = RequestQueue(queue);
    let block_device = &mut *((*queue).queuedata as *mut T);
    block_device.request(&mut rq);
}

pub trait BlockOperations {
    // `const` specifies a compile time constant
    const VTABLE: BlockOperationsVtable;

    const SECTORS: usize;
}

pub trait BlockDeviceDescription {
    fn request(&mut self, queue: &mut RequestQueue);
}

pub enum RequestDirection {
    Read,
    Write,
}

impl Builder {
    pub fn build<T: BlockDeviceDescription + BlockOperations + 'static>(
        self,
        data: T,
    ) -> KernelResult<Registration> {
        let mut device_description: Box<T> = Box::new(data);

        // Allocating major number, simple FFI call.
        let major = unsafe {
            more_bindings::register_blkdev_wrapper(0, self.name.as_ptr() as *const c_types::c_char)
        };

        if major < 0 {
            return Err(Error::from_kernel_errno(major));
        }

        let mut block_device: BlockDevice;

        // Because of BlockDevice::drop, gd and queue MUST be set to null.
        unsafe {
            block_device = BlockDevice {
                gd: null_mut(),
                lock: Box::new(mem::zeroed()),
                queue: null_mut(),
                device_description: None,
            };
        }

        // Initializing a spinlock, simple FFI call.
        unsafe { more_bindings::spin_lock_init_wrapper(block_device.lock.deref_mut()) };
        let mut queue = unsafe {
            more_bindings::blk_init_queue_wrapper(Some(T::VTABLE.1), block_device.lock.deref_mut())
        };
        if queue.is_null() {
            return Err(Error::ENOMEM);
        };
        block_device.queue = queue;

        let mut gd = unsafe { more_bindings::alloc_disk_wrapper(self.minors.len() as i32) };
        if gd.is_null() {
            // Cleanup is done in drop for block_device.
            return Err(Error::ENOMEM);
        };
        block_device.gd = gd;

        // This calls FFI functions which do not return errors.
        unsafe {
            (*queue).queuedata = device_description.as_mut() as *mut T as *mut c_types::c_void;
            (*gd).major = major;
            (*gd).first_minor = self.minors.start as i32;
            let mut blk_fops = T::VTABLE.0.clone();
            // Setting owner prevents removing module, when device is used.
            blk_fops.owner = &mut bindings::__this_module;
            let blk_fops = Box::new(blk_fops);
            (*gd).fops = Box::into_raw(blk_fops);
            (*gd).queue = queue;
            // Many kernel structures have a pointer which can point to anything
            // deemed useful - like custom data structures.
            (*gd).private_data = device_description.as_mut() as *mut T as *mut c_types::c_void;

            // Hand-written setting of name.
            let mut idx = 0;
            for byte in self.name.as_bytes().iter().take(DISK_NAME_LEN - 1) {
                (*gd).disk_name[idx] = *byte as i8;
                idx += 1;
            }
            (*gd).disk_name[idx] = 0_i8;

            more_bindings::set_capacity_wrapper(gd, T::SECTORS as u64);
            more_bindings::add_disk_wrapper(gd);
        }

        block_device.device_description = Some(device_description);

        // Registration object is dropped when module is exitting.
        Ok(Registration {
            dev: major as u32,
            name: self.name,
            _block_device: Some(block_device),
        })
    }
}

pub struct Registration {
    dev: bindings::dev_t,
    name: &'static CStr,
    _block_device: Option<BlockDevice>,
}

unsafe impl Sync for Registration {}

impl Drop for Registration {
    fn drop(&mut self) {
        // Reversing order of drop.
        core::mem::drop(self._block_device.take());
        // Queue is disabled at this point, so it is safe to unregister_blkdev.
        // Order is extremely important here.
        unsafe {
            more_bindings::unregister_blkdev_wrapper(
                self.dev,
                self.name.as_ptr() as *const c_types::c_char,
            );
        }
    }
}

impl BlockOperationsVtable {
    pub const fn builder<T: BlockDeviceDescription>() -> BlockOperationsVtableBuilder<T> {
        BlockOperationsVtableBuilder(
            bindings::block_device_operations {
                // Full list of file operations.
                open: Some(open_callback::<T>),
                release: Some(release_callback::<T>),
                rw_page: None,
                ioctl: None,
                compat_ioctl: None,
                check_events: None,
                media_changed: None,
                unlock_native_capacity: None,
                revalidate_disk: None,
                getgeo: None,
                swap_slot_free_notify: None,
                owner: ptr::null_mut(),
                pr_ops: ptr::null(),
            },
            transfer_callback::<T>,
        )
    }
}

pub struct BlockOperationsVtableBuilder<T>(
    bindings::block_device_operations,
    unsafe extern "C" fn(*mut bindings::request_queue),
);

impl<T> BlockOperationsVtableBuilder<T> {
    pub const fn build(self) -> BlockOperationsVtable {
        BlockOperationsVtable(self.0, self.1)
    }
}

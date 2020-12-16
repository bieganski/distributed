#![no_std]
#![feature(const_fn)]

#[allow(dead_code)]
mod template;

extern crate alloc;

use core::ops::{Deref, DerefMut};
use core::option::Option::Some;
use core::slice::{from_raw_parts, from_raw_parts_mut};

use linux_kernel_module::{self, bindings, c_types, cstr, println, KernelResult};

use crate::blkdev::{
    BlockDeviceDescription, BlockOperations, BlockOperationsVtable, RequestDirection, RequestQueue,
    KERNEL_SECTOR_SIZE,
};

mod blkdev;
#[allow(improper_ctypes)]
mod more_bindings;

struct RamDiskDriver {
    _disk_reg: blkdev::Registration,
}

struct ByteArray {
    data: *mut u8,
    size: usize,
}

impl ByteArray {
    fn try_new(size: usize) -> KernelResult<Self> {
        // Just calling foreign function, if arguments conform to ABI everything will be fine.
        let data = unsafe { more_bindings::vmalloc_wrapper(size as u64) } as *mut u8;

        if data.is_null() {
            return Err(linux_kernel_module::Error::ENOMEM);
        }

        let mut res = ByteArray { data, size };

        for byte in res.deref_mut() {
            *byte = 0_u8;
        }

        Ok(res)
    }
}

impl Drop for ByteArray {
    fn drop(&mut self) {
        // This is OK – memory will be freed once, and this is the only place in code
        // where it happens.
        unsafe { more_bindings::vfree_wrapper(self.data as *mut core::ffi::c_void) };
    }
}

impl Deref for ByteArray {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        // from_raw_parts has docs about what must be satisfied when it is called.
        // The alignment of array of bytes is always OK, and we pass in correct size
        // – there is allocated, owned by us memory there.
        unsafe { from_raw_parts(self.data, self.size) }
    }
}

impl DerefMut for ByteArray {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Similar as deref.
        unsafe { from_raw_parts_mut(self.data, self.size) }
    }
}

unsafe extern "C" fn transfer_request_block_callback(
    rq: *mut bindings::request,
    sector: u64,
    len: usize,
    buf: *mut c_types::c_uchar,
    direction: i32,
) {
    let block_device = &mut *((*(*rq).q).queuedata as *mut RamDisk);
    let direction = match direction {
        0 => RequestDirection::Read,
        1 => RequestDirection::Write,
        _ => panic!("Cannot happen, third direction??"),
    };
    block_device.transfer(sector, from_raw_parts_mut(buf, len), direction);
}

struct RamDisk {
    data: ByteArray,
}

impl RamDisk {
    fn try_new() -> KernelResult<Self> {
        let data = ByteArray::try_new((KERNEL_SECTOR_SIZE as usize * Self::SECTORS) as usize)?;

        Ok(RamDisk { data })
    }

    fn transfer(&mut self, sector: u64, buffer: &mut [u8], direction: RequestDirection) {
        let offset: usize = (sector * KERNEL_SECTOR_SIZE as u64) as usize;
        let len = buffer.len();

        if (offset + len) > self.data.size {
            return;
        }

        match direction {
            RequestDirection::Read => {
                buffer.copy_from_slice(&self.data.deref()[offset..(offset + len)]);
            }
            RequestDirection::Write => {
                self.data.deref_mut()[offset..(offset + len)].copy_from_slice(buffer);
            }
        }
    }
}

impl BlockOperations for RamDisk {
    const VTABLE: BlockOperationsVtable = BlockOperationsVtable::builder::<RamDisk>().build();

    const SECTORS: usize = 2048;
}

const BLK_STS_IOERR: u8 = 10;

impl BlockDeviceDescription for RamDisk {
    fn request(&mut self, queue: &mut RequestQueue) {
        while let Some(mut rq) = queue.fetch_request() {
            if rq.is_passthrough() {
                rq.set_error(BLK_STS_IOERR);
                println!("skipping non-fs request");
                continue;
            }

            // This is where API of blkdev is not ideal. But again, simple FFI here.
            unsafe {
                more_bindings::xfer_request(Some(transfer_request_block_callback), rq.request)
            }
        }
    }
}

impl linux_kernel_module::KernelModule for RamDiskDriver {
    fn init() -> KernelResult<Self> {
        let ram_disk = RamDisk::try_new()?;
        Ok(RamDiskDriver {
            _disk_reg: blkdev::builder(cstr!("rdc"), 0..1)?.build(ram_disk)?,
        })
    }
}

linux_kernel_module::kernel_module!(
    RamDiskDriver,
    author: b"N.N.",
    description: b"A module providing RAM disc",
    license: b"GPL"
);

use linux_kernel_module::{bindings, c_types};

extern "C" {
    pub fn blk_rq_is_passthrough_wrapper(_rq: *mut bindings::request) -> c_types::c_uchar;

    pub fn set_capacity_wrapper(_disk: *mut bindings::gendisk, _size: bindings::sector_t);

    pub fn alloc_disk_wrapper(_minors: c_types::c_int) -> *mut bindings::gendisk;

    pub fn spin_lock_init_wrapper(_lock: *mut bindings::spinlock_t);

    pub fn add_disk_wrapper(_gd: *mut bindings::gendisk);

    pub fn __blk_end_request_all_wrapper(_rq: *mut bindings::request, error: c_types::c_uchar);

    pub fn del_gendisk_wrapper(_gd: *mut bindings::gendisk);

    pub fn put_disk_wrapper(_gd: *mut bindings::gendisk);

    pub fn blk_fetch_request_wrapper(_rq: *mut bindings::request_queue) -> *mut bindings::request;

    pub fn register_blkdev_wrapper(major: usize, name: *const c_types::c_char) -> i32;

    pub fn blk_init_queue_wrapper(
        fun: Option<unsafe extern "C" fn(*mut bindings::request_queue)>,
        _lock: *mut bindings::spinlock_t,
    ) -> *mut bindings::request_queue;

    pub fn blk_cleanup_queue_wrapper(_rq: *mut bindings::request_queue);

    pub fn unregister_blkdev_wrapper(major: u32, name: *const c_types::c_char);

    pub fn vmalloc_wrapper(_size: u64) -> *mut c_types::c_void;

    pub fn vfree_wrapper(_ptr: *const c_types::c_void);

    pub fn xfer_request(
        call: Option<
            unsafe extern "C" fn(
                rq: *mut bindings::request,
                sector: u64,
                len: usize,
                buf: *mut c_types::c_uchar,
                direction: i32,
            ),
        >,
        _req: *mut bindings::request,
    );
}

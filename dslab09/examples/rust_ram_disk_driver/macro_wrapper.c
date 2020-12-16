#include <linux/highmem.h>
#include <linux/blkdev.h>
#include <linux/genhd.h>
#include <linux/spinlock.h>

int rq_data_dir_wrapper(struct request *request)
{
    return rq_data_dir(request);
}

bool blk_rq_is_passthrough_wrapper(struct request *request)
{
    return blk_rq_is_passthrough(request);
}

sector_t blk_rq_pos_wrapper(struct request *request)
{
    return blk_rq_pos(request);
}

unsigned long blk_rq_bytes_wrapper(struct request *request)
{
    return blk_rq_bytes(request);
}

void set_capacity_wrapper(struct gendisk *disk, sector_t size)
{
    return set_capacity(disk, size);
}

struct gendisk *alloc_disk_wrapper(int minors)
{
    return alloc_disk(minors);
}

void spin_lock_init_wrapper(spinlock_t *lock)
{
    spin_lock_init(lock);
}

void add_disk_wrapper(struct gendisk *disk)
{
    add_disk(disk);
}

void __blk_end_request_all_wrapper(struct request *rq, blk_status_t error)
{
    __blk_end_request_all(rq, error);
}

void del_gendisk_wrapper(struct gendisk *gp)
{
    del_gendisk(gp);
}

void put_disk_wrapper(struct gendisk *disk)
{
    put_disk(disk);
}

void blk_cleanup_queue_wrapper(struct request_queue *rq)
{
    blk_cleanup_queue(rq);
}

struct request *blk_fetch_request_wrapper(struct request_queue *q)
{
    return blk_fetch_request(q);
}

int register_blkdev_wrapper(unsigned int major, const char *name)
{
    return register_blkdev(major, name);
}

struct request_queue *blk_init_queue_wrapper(request_fn_proc *fun, spinlock_t *lock)
{
    return blk_init_queue(fun, lock);
}

void unregister_blkdev_wrapper(unsigned int major, const char *name)
{
    unregister_blkdev(major, name);
}

void *vmalloc_wrapper(unsigned long size)
{
    return vmalloc(size);
}

void vfree_wrapper(const void *addr)
{
    vfree(addr);
}

void xfer_request(void (*transfer_callback(struct request *req, sector_t, size_t, char *, int)), struct request *req)
{
    struct bio_vec bvec;
    struct req_iterator iter;

    rq_for_each_segment(bvec, req, iter) {
        sector_t sector = iter.iter.bi_sector;
        unsigned long offset = bvec.bv_offset;
        size_t len = bvec.bv_len;
        int dir = bio_data_dir(iter.bio);
        /* Requests to block devices contain a set of `struct page`, which describes physical memory location
           of buffer. That is why it as to be mapped with `kmap_atomic`. The `_atomic` means it is not allowed
           to sleep after calling this function, before `kunmap_atomic` is called. */
        char *buffer = kmap_atomic(bvec.bv_page);

        (*transfer_callback)(req, sector, len, buffer + offset, dir);
        kunmap_atomic(buffer);
    }
}

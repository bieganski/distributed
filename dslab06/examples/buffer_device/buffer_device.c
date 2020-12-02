#include <linux/init.h>
#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/fs.h>
#include <linux/uaccess.h>
#include <linux/string.h>

MODULE_LICENSE("Dual MIT/GPL");
MODULE_AUTHOR("N.N.");
MODULE_DESCRIPTION("A simple driver for data buffer. Write stores data to global buffer, "
                   "while read returns it.");
MODULE_VERSION("0.42");

#define DEVICE_NAME "ds_buf"
#define MSG_BUFFER_LEN 32

/* Prototypes for device functions. */
static int device_open(struct inode *, struct file *);
static int device_release(struct inode *, struct file *);
static ssize_t device_read(struct file *, char *, size_t, loff_t *);
static ssize_t device_write(struct file *, const char *, size_t, loff_t *);
static loff_t device_lseek(struct file *file, loff_t offset, int orig);
static int major_num;

/* In this table data provided from userspace is stored, so it can be read later on. */
static char buffered_data[MSG_BUFFER_LEN];

/* This creates a mutex. We will use one for the whole driver. */
DEFINE_MUTEX(driver_lock);

/* This structure points to all of the device functions. */
static struct file_operations file_ops = {
        // This field is for bumping usage count of module when a file is opened.
        // After releasing, count is decreased - this prevents unloading of the
        // driver when a there is an file descriptor in usage.
        .owner = THIS_MODULE,
        .read = device_read,
        .write = device_write,
        .open = device_open,
        // After last close has been called on the file.
        .release = device_release,
        .llseek = device_lseek
};

/* When a process reads from our device, this gets called. Note the special
 * marker for user space pointer. */
static ssize_t device_read(struct file *flip, char __user *buffer, size_t len, loff_t *offset) {
    int error = 0;
    loff_t pos = *offset;

    if (pos >= MSG_BUFFER_LEN || pos < 0) {
        return 0;
    }

    if (len > MSG_BUFFER_LEN - pos)
        len = MSG_BUFFER_LEN - pos;

    /* A lot of kernel code acquires a few resources to perform some task. To handle errors,
     * this is often coded as a sequence of goto's to subsequent layers that perform more cleaning. */
    if ((error = mutex_lock_interruptible(&driver_lock)))
        goto err_driver_lock;

    /* Copying from kernel to user space must not be done directly! An attempt to access user
     * space directly may just fail or, even worse, cause serious security issues. */
    if (copy_to_user(buffer, buffered_data + pos, len)) {
        /* Errors in kernel are indicated with negative numbers */
        error = -EFAULT;
        goto err_copy;
    }

    err_copy:
    mutex_unlock(&driver_lock);

    err_driver_lock:

    /* Either error code is returned-when no data was processed-or the number of processed bytes.
     * It can be lower then request length. */
    if (error) {
        return error;
    } else {
        *offset = pos + len;
        return len;
    }
}

/* Called when a process tries to write to our device. */
static ssize_t device_write(struct file *flip, const char *buffer, size_t len, loff_t *offset) {
    int error = 0;
    loff_t pos = *offset;

    if (pos >= MSG_BUFFER_LEN || pos < 0) {
        return -EINVAL;
    }

    if (len > MSG_BUFFER_LEN - pos)
        len = MSG_BUFFER_LEN - pos;

    /* Kernel also provides spinlocks, but they can only be used to synchronize actions
     * which take a brief time. Copy_{to, from}_user can put process to sleep, which means
     * using a mutex is necessary to save CPU time. */
    if ((error = mutex_lock_interruptible(&driver_lock)))
        goto err_driver_lock;

    /* Copying to kernel also must be done via special function. */
    if ((error = copy_from_user(buffered_data + pos, buffer, len)))
        goto err_copy;

    err_copy:
    mutex_unlock(&driver_lock);

    err_driver_lock:

    if (error) {
        return error;
    } else {
        *offset = pos + len;
        return len;
    }
}

static loff_t device_lseek(struct file *file, loff_t offset, int orig)
{
    loff_t new_pos;

    switch (orig) {
        case SEEK_SET:        /* Seek set. */
            new_pos = offset;
            break;
        case SEEK_CUR:        /* Seek cur. */
            new_pos = file->f_pos + offset;
            break;
        case SEEK_END:        /* Seek end. */
            new_pos = MSG_BUFFER_LEN - offset;
            break;
        default:
            return -EINVAL;
    }
    if (new_pos > MSG_BUFFER_LEN)
        new_pos = MSG_BUFFER_LEN;
    if (new_pos < 0)
        new_pos = 0;
    file->f_pos = new_pos;
    return new_pos;
}

/* Called when a process opens our device. */
static int device_open(struct inode *inode, struct file *file) {
    return 0;
}

/* Called when a process closes our device. */
static int device_release(struct inode *inode, struct file *file) {
    return 0;
}

static int __init buffer_init(void) {
    memset(buffered_data, 'a', MSG_BUFFER_LEN);
    /* Try to register character device. */
    major_num = register_chrdev(0, DEVICE_NAME, &file_ops);
    if (major_num < 0) {
        printk(KERN_ALERT "Could not register device: %d\n", major_num);
        return major_num;
    } else {
        printk(KERN_INFO "buffer module loaded with device major number %d\n", major_num);
        return 0;
    }
}

static void __exit buffer_exit(void) {
    /* Remember — we have to clean up after ourselves. Unregister the character device. */
    unregister_chrdev(major_num, DEVICE_NAME);
    printk(KERN_INFO "Goodbye, World!\n");
}

/* Register module functions. */
module_init(buffer_init);
module_exit(buffer_exit);

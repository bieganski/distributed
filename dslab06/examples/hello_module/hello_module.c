#include <linux/init.h>
#include <linux/module.h>
#include <linux/kernel.h>

MODULE_LICENSE("Dual MIT/GPL");
MODULE_AUTHOR("N.N.");
MODULE_DESCRIPTION("Hello Linux modules!");
MODULE_VERSION("0.42");

static int __init hello_init(void) {
    printk(KERN_INFO "Hello, World!\n");
    return 0;
}
static void __exit hello_exit(void) {
    printk(KERN_INFO "Goodbye, World!\n");
}
module_init(hello_init);
module_exit(hello_exit);

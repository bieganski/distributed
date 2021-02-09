#!/bin/bash

CORES=2

qemu-system-x86_64 \
    -device virtio-scsi-pci,id=scsi0 \
    -drive file=ds2020_cow.img,if=none,id=drive0 \
    -device scsi-hd,bus=scsi0.0,drive=drive0 \
    -smp $CORES \
    -net nic,model=virtio \
    -net user,hostfwd=tcp::2222-:22 \
    -enable-kvm \
    -display none \
    -chardev stdio,id=cons,signal=off \
    -device virtio-serial-pci \
    -device virtconsole,chardev=cons \
    -fsdev local,id=hshare,path=./hshare,security_model=none \
    -device virtio-9p-pci,fsdev=hshare,mount_tag=hshare \
    -m 1G

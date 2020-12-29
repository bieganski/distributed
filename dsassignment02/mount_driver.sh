#!/bin/bash

set -x

echo "major: $major..."

make -B
sudo ./sample_insmod.sh


major=`dmesg | tail | grep "atdd:" | grep major | tail -n 1 | rev | cut -d ' ' -f 1 | rev`

if [ "$major" == "" ]; then
	echo "error! major not found..."
	exit 1
fi

rm -rf /dev/lol
mknod /dev/lol b $major 0
sudo chown ds /dev/lol
ls -l /dev/lol
# sudo mkfs -t ext4 /dev/lol
# sudo mount -t ext4 /dev/lol test

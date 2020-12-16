#!/bin/bash

USER=mb385162
set -u
set -x
function log {
	echo "==== $@"
}

if [ ! -d ./solution ]; then
	log "no solution dir!"
	exit 0
fi


TARGET=solution
TCP=$USER/$TARGET.copy
mkdir -p $USER
cp -r $TARGET $TCP

rm -rf $TCP/target
zip -r $USER.zip $TCP


#!/bin/bash

# set -x
set -e
# set -u

function log {
	echo "==== $@"
}

function usage {
	me=`basename $0`
	echo "usage: $me
		<number of lab>
		--unpack (download and push as template)
		--dispatch (make it ready to dispatch (zip))"
}

function unpack {
	base=dslab$NUM
	file=~/Downloads/$base.tgz
	if ! [ -f $file ]; then
		log "Error: $file not found!"
		exit 1
	fi
	tar -xvf $file -C ~/distributed/
	cd ~/distributed/$base
	cargo build
	git add .
	git commit -m "solution template $base added"
	git push
}

function dispatch {
	base=dslab0$NUM
	cd ~/distributed/$base/
	mkdir mb385162
	cp solution.rs mb385162
	zip -r mb385162.zip mb385162/
	rm -rf mb385162
	mv mb385162.zip ~/distributed
}

if [[ "$1" == "" ]]; then
	usage
	exit 1
fi

NUM=
re='^[0-9]+$' # number
if ! [[ $1 =~ $re ]] ; then
	log "Error: first argument is not not a number!"
	usage
	exit 1
else
	NUM=$1
fi
shift

while ! [[ "$1" == "" ]]; do 
case "$1" in
	--unpack)
		unpack
		exit 0
		;;
	--dispatch)
		dispatch
		exit 0
		;;
	*)
		log "Error: argument $1 unknown! Exiting..."
		usage
		exit 1
		;;
esac
shift
done

log "Error: specify --unpack or --dispatch argument!"
usage



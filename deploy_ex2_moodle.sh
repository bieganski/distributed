#!/bin/bash

set -e
set -x
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# run in ~/temp/distributed

# dot '.' stands for * in regex language.
if ! [[ $SCRIPT_DIR =~ ./temp/distributed ]]; then
	echo "--- error: wrong path"
	exit 1
fi



rm -rf mb385162
pushd ~/distributed/ > /dev/null
rm -f mb385162.zip
./large_devops.sh 2 --dispatch

if ! [ -f mb385162.zip ]; then
	echo "---- error! .zip wanst created!"
	exit 1
fi

mv mb385162.zip ~/temp/distributed/

popd > /dev/null


if ! [ -f mb385162.zip ]; then
	echo "---- error! .zip wasnt copied!"
	exit 1
fi

unzip mb385162.zip

cp -r mb385162/solution/* dsassignment02/solution

pushd dsassignment02/public-tests > /dev/null

cargo test
popd > /dev/null

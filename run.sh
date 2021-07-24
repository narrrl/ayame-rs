#!/bin/bash

ABSPATH="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"

cd $ABSPATH
if [ "$1" == "start" ]; then
	if [[ ! -f "$FILE" ]]; then
		cargo build --release
		cp ./target/release/nirust .
	fi
	./nirust & echo $! > ./nirust.pid
elif [ "$1" == "stop" ]; then
	kill -SIGINT "$(cat ./nirust.pid)"
else
	printf "No argument provided\n"
	exit -1
fi
exit 0

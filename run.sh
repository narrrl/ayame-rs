#!/bin/bash

if [ "$1" == "start" ]; then
	/opt/nirust/nirust & echo $! > ./nirust.pid
elif [ "$1" == "stop" ]; then
	kill -SIGINT "$(cat ./nirust.pid)"
else
	printf "No argument provided\n"
	exit -1
fi
exit 0

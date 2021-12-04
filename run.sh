#!/bin/bash

ABSPATH="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"
FILE="ayame-rs"
PIDF="ayame.pid"
CONFIG="config.toml"

function main() {
	if [ "$1" == "start" ]; then
		if [[ -f "$PIDF" ]]; then
			printf "Ayame already running\n"
			exit -1
		fi
		if [[ ! -f "$FILE" ]]; then
			cargo build --release
			cp ./target/release/$FILE .
		fi
		./$FILE& echo $! > ./$PIDF
	elif [ "$1" == "stop" ]; then
		kill -SIGINT "$(cat $PIDF)"
		rm $PIDF
	elif [ "$1" == "update" ]; then
		kill -SIGINT "$(cat $PIDF)"
		rm ./$PIDF
		rm ./$FILE
		git fetch --all && git reset --hard && git clean -fd
		git pull
		cargo build --release
		cp ./target/release/$FILE .
		./$FILE & echo $! > ./$PIDF
	else
		printf "No argument provided\n"
		exit -1
	fi
}

function check_config() {
	if [[ ! -f "$CONFIG" ]]; then
		printf "Your bot token: "
		read TOKEN
		echo "token = \"$TOKEN\"" > $CONFIG
		printf "\nYour application id (usually your bot user id): "
		read APP_ID
		echo "application_id = $APP_ID" >> $CONFIG
		printf "\nYour prefix: "
		read PREFIX
		echo "prefix = \"$PREFIX\"" >> $CONFIG
	fi

}

cd $ABSPATH
check_config
main $1

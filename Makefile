CONTAINER_NAME := ayame
SOURCE := .
CONTAINER_ID := $(shell docker ps --latest -f 'name=${CONTAINER_NAME}' -q)
DATABASE_URL := sqlite:database/database.sqlite

.PHONY: build
build:
	docker build -t ${CONTAINER_NAME} ${SOURCE}
ifneq ($(strip $(shell docker images --filter "dangling=true" -q --no-trunc)),)
	docker rmi $(shell docker images --filter "dangling=true" -q --no-trunc)
endif

.PHONY: run
run:
	docker run --name ${CONTAINER_NAME} -d ${CONTAINER_NAME}:latest

.PHONY: start
start:
	docker start ${CONTAINER_NAME}

.PHONY: stop
stop:
	docker stop ${CONTAINER_NAME}

.PHONY: prepare
prepare:
	DATABASE_URL=${DATABASE_URL} cargo sqlx prepare --database-url ${DATABASE_URL}


.PHONY: clean
clean:
ifneq ($(shell type cargo),)
	cargo clean
endif
ifneq ($(strip $(CONTAINER_ID)),)
	docker rm ${CONTAINER_ID}
endif
ifneq ($(strip $(shell docker images ${CONTAINER_NAME})),)
	docker rmi ${CONTAINER_NAME}
endif

all: build run

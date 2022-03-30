CONTAINER_NAME := ayame
SOURCE := .
CONTAINER_ID := $(shell docker ps --latest -f 'name=${CONTAINER_NAME}' -q)


.PHONY: build
build:
	docker build -t ${CONTAINER_NAME} ${SOURCE}
ifneq ($(strip $(shell docker images --filter "dangling=true" -q --no-trunc)),)
	docker rmi $(shell docker images --filter "dangling=true" -q --no-trunc)
endif

.PHONY: run
run:
	docker run --name ${CONTAINER_NAME} -d ${CONTAINER_NAME}:latest

.PHONY: clean
clean:
	cargo clean
ifneq ($(strip $(CONTAINER_ID)),)
	docker rm ${CONTAINER_ID}
endif
ifneq ($(strip $(shell docker images ${CONTAINER_NAME})),)
	docker rmi ${CONTAINER_NAME}
endif

all: build run

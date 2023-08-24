#!/bin/bash

THIS_DIRECTORY=`realpath $(dirname "$0")`

# Assuming docker and buildx installed

export DOCKER_CLI_EXPERIMENTAL=enabled
export DOCKER_BUILDKIT=1

docker run --rm --privileged docker/binfmt:820fdd95a9972a5308930a2bdfb8573dd4447ad3 

# cat /proc/sys/fs/binfmt_misc/qemu-aarch64 # Check this is enabled

docker buildx create --name rust-compiler || true # May fail if already existing
docker buildx use rust-compiler
# docker buildx inspect --bootstrap # Check everything is ok

# Does this need a --platform linux/arm64??
# Run with --no-cache if needed
docker buildx build -t "sirgovan-compiler" ${THIS_DIRECTORY}/../docker --load $1
docker tag sirgovan-compiler ghcr.io/sergiovan/sirgovan-compiler:latest
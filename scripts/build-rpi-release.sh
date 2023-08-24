#!/bin/bash

THIS_DIRECTORY=`realpath $(dirname "$0")`

docker run --rm --name sirgovan-compilation-release \
       -v $THIS_DIRECTORY/..:/sirgovan-rust/ \
       -u $(id -u ${USER}):$(id -g ${USER}) \
       ghcr.io/sergiovan/sirgovan-compiler:latest --release
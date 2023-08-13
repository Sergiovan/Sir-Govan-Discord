#!/bin/bash

THIS_DIRECTORY=`realpath $(dirname "$0")`

docker run --rm --name sirgovan-compilation-debug -v $THIS_DIRECTORY/..:/sirgovan-rust/ ghcr.io/sergiovan/sirgovan-compiler:latest
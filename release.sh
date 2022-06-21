#!/bin/sh

docker build -f Dockerfile.cc -t zten/rpxc .
docker run -it --rm -v $PWD:/build zten/rpxc cargo build --release
scp target/aarch64-unknown-linux-gnu/release/rpi pi:/home/dubba/rpi

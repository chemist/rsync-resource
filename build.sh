#!/bin/bash

docker run --rm -it -v "$(pwd)":/home/rust/src ekidd/rust-musl-builder cargo build --release
#rust-musl-builder cargo build --release

docker  build -t chemist/rsync-resource  .
docker push chemist/rsync-resource

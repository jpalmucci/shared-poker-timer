#!/bin/sh

set -e
cargo leptos build --release --precompress
docker build -t jpalmucci/pokertimer:latest .
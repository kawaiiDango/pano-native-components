#!/bin/bash

cargo build --release
cp target/release/libnative_components.so ../pano-scrobbler/composeApp/resources/linux/

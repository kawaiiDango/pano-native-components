#!/bin/bash

cargo build --release
cp target/release/libnative_components.so ../pano-scrobbler/composeApp/resources/linux-x64/;
cp target/release/libnative_components.dylib ../pano-scrobbler/composeApp/resources/macos-x64/

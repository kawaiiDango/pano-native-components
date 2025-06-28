#!/bin/bash

cargo build --release
cp target/release/libpano_native_components.so ../pano-scrobbler/composeApp/resources/linux-x64/;
cp target/release/libnative_webview.so ../pano-scrobbler/composeApp/resources/linux-x64/;

# Native Components for Pano Scrobbler

A JNI lib, used for accessing some native APIs on desktop, that are not possible with JVM.

### Build

```sh
cargo build --release
```

Linux may need distro specific dev dependencies for webkit6

### Test

```sh
javac -h . PanoNativeComponents.java

cargo build --release && javac -d . PanoNativeComponents.java && java -Djava.library.path=target/release/ com.arn.scrobble.PanoNativeComponents
```

or (if Powershell)

```sh
cargo build --release && javac -d . PanoNativeComponents.java && java "-Djava.library.path=target/release/" com.arn.scrobble.PanoNativeComponents
```

### Package

```sh
cp target/release/pano_native_components.dll pano-scrobbler-dir/composeApp/resources/windows-x64/
cp target/release/native_webview.dll pano-scrobbler-dir/composeApp/resources/windows-x64/
```

```sh
cp target/release/libpano_native_components.so pano-scrobbler-dir/composeApp/resources/linux-x64/
cp target/release/libnative_webview.so pano-scrobbler-dir/composeApp/resources/linux-x64/
```


I used code from these projects as a reference:

https://github.com/Mange/mpris-rs

https://github.com/KDE/kdeconnect-kde


### License

SPDX-License-Identifier: GPL-3.0-or-later
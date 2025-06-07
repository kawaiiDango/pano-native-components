# Native Components for Pano Scrobbler

A JNI lib, used for accessing some native APIs on desktop, that are not possible with JVM.


This project was compiled with rust nightly.
It may use features that are not available in rust stable.


### Build

```
cargo build --release
```

Linux needs additional dependencies listed at [tauri-apps/wry](https://github.com/tauri-apps/wry)

### Test

```
javac -h . PanoNativeComponents.java

cargo build --release && javac -d . PanoNativeComponents.java && java -Djava.library.path=target/release/ com.arn.scrobble.PanoNativeComponents
```

or (if Powershell)

```
cargo build --release && javac -d . PanoNativeComponents.java && java "-Djava.library.path=target/release/" com.arn.scrobble.PanoNativeComponents
```

### Package

```
cp target/release/pano_native_components.dll pano-scrobbler-dir/composeApp/resources/windows-x64/
```

```
cp target/release/libpano_native_components.so pano-scrobbler-dir/composeApp/resources/linux-x64/
```


I used code from these projects as a reference:

https://github.com/Mange/mpris-rs

https://github.com/KDE/kdeconnect-kde
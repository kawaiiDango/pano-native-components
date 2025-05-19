# Native Components for Pano Scrobbler

A JNI lib, used for accessing some native APIs on desktop, that are not possible with JVM.

Linux needs additional dependencies listed at [tauri-apps/wry](https://github.com/tauri-apps/wry)

cargo build --release



To test: 

javac -h . PanoNativeComponents.java

cargo build --release && javac -d . PanoNativeComponents.java && java -Djava.library.path=target/release/ com.arn.scrobble.PanoNativeComponents

or (if powershell)

cargo build --release && javac -d . PanoNativeComponents.java && java "-Djava.library.path=target/release/" com.arn.scrobble.PanoNativeComponents


To package:

cp target/release/native_components.dll pano-scrobbler-dir/composeApp/resources/windows-x64/
cp target/release/libnative_components.so pano-scrobbler-dir/composeApp/resources/linux-x64/



Projects I used as a reference:

https://github.com/Mange/mpris-rs

https://github.com/KDE/kdeconnect-kde
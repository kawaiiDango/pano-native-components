# Native Components for Pano Scrobbler

Attributions:

https://github.com/Mange/mpris-rs

https://github.com/KDE/kdeconnect-kde

(if building for linux)

sudo pacman -S gtk3

or

sudo apt install libgtk-3-dev

cargo build --release



To test: 

javac -h . PanoNativeComponents.java

cargo build --release && javac -d . PanoNativeComponents.java && java -Djava.library.path=target/release/ com.arn.scrobble.PanoNativeComponents

or (if powershell)

cargo build --release && javac -d . PanoNativeComponents.java && java "-Djava.library.path=target/release/" com.arn.scrobble.PanoNativeComponents


To package:

cp target/release/native_components.dll pano-scrobbler-dir/composeApp/resources/windows-x64/
cp target/release/libnative_components.so pano-scrobbler-dir/composeApp/resources/linux-x64/

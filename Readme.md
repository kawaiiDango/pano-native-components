# Native Components for Pano Scrobbler

Attributions:

https://github.com/Mange/mpris-rs

https://github.com/KDE/kdeconnect-kde

(if building for linux)

sudo pacman -S gtk3 libayatana-appindicator

or

sudo apt install libgtk-3-dev libayatana-appindicator3-dev


javac -h . PanoNativeComponents.java

cargo build --release


To test: 

cargo build --release && javac -d . PanoNativeComponents.java && java -Djava.library.path=target/release/ com.arn.scrobble.PanoNativeComponents

or (if powershell)

cargo build --release && javac -d . PanoNativeComponents.java && java "-Djava.library.path=target/release/" com.arn.scrobble.PanoNativeComponents


To package:

cp target/release/native_components.dll pano-scrobbler-dir/composeApp/resources/windows/
cp target/release/libnative_components.so pano-scrobbler-dir/composeApp/resources/linux/

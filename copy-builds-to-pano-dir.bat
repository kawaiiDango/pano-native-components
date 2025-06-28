cargo build --release
copy target\release\pano_native_components.dll ..\pano-scrobbler\composeApp\resources\windows-x64\
copy target\release\native_webview.dll ..\pano-scrobbler\composeApp\resources\windows-x64\

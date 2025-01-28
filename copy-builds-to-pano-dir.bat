cargo build --release
copy release\libnative_components.so ..\pano-scrobbler\composeApp\resources\linux-x64\
copy target\release\native_components.dll ..\pano-scrobbler\composeApp\resources\windows-x64\

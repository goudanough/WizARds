#!/bin/sh
mkdir -p runtime_libs/arm64-v8a/

#create temp dir to download zip file to
tmp_dir=$(mktemp -d)

# OpenXR SDK 60.0, update link as necessary
curl 'https://securecdn.oculus.com/binaries/download/?id=7092833820755144' --output "$tmp_dir/oxr_sdk.zip"
# move the library into the necessary folder for our project
unzip -j "$tmp_dir/oxr_sdk.zip" OpenXR/Libs/Android/arm64-v8a/Release/libopenxr_loader.so -d runtime_libs/arm64-v8a

# Platform SDK 60.0, update link as necessary
curl 'https://securecdn.oculus.com/binaries/download/?id=5285000204956972' --output "$tmp_dir/platform_sdk.zip"
# move the library into the necessary folder for our project
unzip -j "$tmp_dir/platform_sdk.zip" Android/libs/arm64-v8a/libovrplatformloader.so -d runtime_libs/arm64-v8a

# Vosk library 
wget 'https://github.com/alphacep/vosk-api/releases/download/v0.3.45/vosk-android-0.3.45.zip' -O "$tmp_dir/vosk.zip"
# move the library into the necessary folder for our project
unzip -j "$tmp_dir/vosk.zip" arm64-v8a/libvosk.so -d runtime_libs/arm64-v8a

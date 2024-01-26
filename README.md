# wizARds

## Setup
Get libopenxr_loader.so from the Oculus OpenXR Mobile SDK and add it to a new folder `runtime_libs/arm64-v8a/`
https://developer.oculus.com/downloads/package/oculus-openxr-mobile-sdk/ \
The shell script `get_oxr_loader.sh` has been provided for automating this for Linux/Mac users

Compiling will be done with `xbuild`, which can be installed with the following: \
(Note that the `--git` is very important here)
```sh
cargo install --git https://github.com/rust-mobile/xbuild
```
(Make sure that xbuild is installed to your $PATH so you can access it via the command line)

You can check if you've got all the necessary dependencies to use xbuild by running
```sh
x doctor
```
The most important components to have installed are ADB and LLVM

## Run
Running on Meta Quest can be done with xbuild:
```sh
# List devices and copy device string "adb:***"
x devices

# Run on this device
x run --release --device adb:***
```
Note that enumerating the devices may error with "insufficient permissions for device". This can be fixed on Linux by running 
```sh
adb kill-server && sudo adb start-server
```

[manifest.yaml](./manifest.yaml) is required by xbuild to enable permissions in Android.
Interface for this manifest can be found as AndroidConfig struct in https://github.com/rust-mobile/xbuild/blob/master/xbuild/src/config.rs

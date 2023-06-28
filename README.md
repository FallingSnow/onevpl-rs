### work in progress

# oneVPL
oneVPL is Intel's replacement for intel-media-sdk.

You should really only use this with intel hardware acceleration. The onevpl CPU runtime is End-of-Life.

## Features
- [x] Software Encode
- [x] Hardware Encode
- [x] Software Decode
- [x] Hardware Decode
- [x] Simple Video Post/Pre Processing (VPP)
    - Color space conversion, Crop
- [ ] Advanced Video Post/Pre Processing (VPP)
    - Sharpening, Denoise, Rotate, etc.
- [ ] External Frame Allocator (Use your own buffers)
- [ ] Legacy API

## Dependencies
Building bindings requires clang to be installed.
* [clang](https://rust-lang.github.io/rust-bindgen/requirements.html)
* [oneVPL](https://www.intel.com/content/www/us/en/developer/articles/tool/oneapi-standalone-components.html#onevpl)

### Windows
If you install clang tools with VS build tools (not recommended, use link above instead), you may have to manually set the libclang path with an environmental variable in order to build. The folder should contain `libclang.dll`. For example in powershell:
```
$env:LIBCLANG_PATH="C:/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/VC/Tools/Llvm/x64/bin"
cargo build
```

## Usage
See examples folder. You need git lfs to download the test files. You may need to run `git lfs install` after cloning.

### Examples

#### Decode a file
```
RUST_LOG=trace cargo run --example decode_file
```

#### Encode a file
```
RUST_LOG=trace cargo run --example encode_file
```

## Notes
- HW encoding requires HW input formats (NV12 instead of YUV). You should use the VPP to preprocess the video/frames into HW formats. See `encode_file_hw` example.

## Thread Safety
It appears the OneVPL is thread safe in the "main loop" of the application.
https://community.intel.com/t5/Media-Intel-oneAPI-Video/oneVPL-beta10-concurrent-encode-stream/m-p/1233576

This library does it's best to try to enforce that. If you find a theading bug, please open an issue.

## Todo
- Encode
- Decode
    - [ ] Example to decode to RGB4
        - [ ] Example to decode to drm
    - [x] Simple hardware accelerated decoding
        - https://spec.oneapi.io/versions/latest/elements/oneVPL/source/programming_guide/VPL_prg_hw.html
    - [ ] 10 bit decoding support
        - Mostly just need to write the functions to write pl10 to a file
- VPP
    - [x] Simple VPP processing
        - [ ] Pipelined VPP processing https://spec.oneapi.io/versions/latest/elements/oneVPL/source/API_ref/VPL_func_vid_decode_vpp.html
- [ ] Write tests


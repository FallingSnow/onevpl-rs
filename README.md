### work in progress

# oneVPL
oneVPL is Intel's replacement for intel-media-sdk.

## Features
- [ ] Encode
- [x] Decode
- [ ] Video Post/Pre Processing (VPP)


## Usage
See examples folder.


### Examples

#### Decode a file
```
RUST_LOG=trace cargo run --example decode_file
```

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
    - [ ] Simple VPP processing
        - [ ] Pipelined VPP processing https://spec.oneapi.io/versions/latest/elements/oneVPL/source/API_ref/VPL_func_vid_decode_vpp.html
- [ ] Write tests
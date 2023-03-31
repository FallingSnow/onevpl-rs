### work in progress

# oneVPL
oneVPL is Intel's replacement for intel-media-sdk.


## Usage
See examples folder.


### Examples

#### Decode a file
```
RUST_LOG=trace cargo run --example decode_file
```

## Todo
- [ ] Example to decode to RGB4
    - [ ] Example to decode to drm
- [x] Simple hardware accelerated decoding
    - https://spec.oneapi.io/versions/latest/elements/oneVPL/source/programming_guide/VPL_prg_hw.html
- [ ] 10 bit decoding support
    - Mostly just need to write the functions to write pl10 to a file
- [ ] Simple VPP processing
    - [ ] Pipelined VPP processing
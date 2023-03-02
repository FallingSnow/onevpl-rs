use std::io;

use intel_onevpl_sys::MfxStatus;
use onevpl::{self, init, Loader, Bitstream, constants};

const DEFAULT_BUFFER_SIZE: usize = 1024 * 1024 * 2; // 2MB

pub fn main() {
    init().unwrap();

    // Open file to read from
    let mut file = std::fs::File::open("tests/frozen.hevc").unwrap();
    let mut output = std::fs::File::create("/tmp/output.yuv").unwrap();

    let mut loader = Loader::new().unwrap();

    let config = loader.new_config().unwrap();
    // Set software decoding
    config
        .set_filter_property_u32("mfxImplDescription.Impl", constants::Impl::Software.repr(), None)
        .unwrap();

    let config = loader.new_config().unwrap();
    // Set decode HEVC
    config
        .set_filter_property_u32(
            "mfxImplDescription.mfxDecoderDescription.decoder.CodecID",
            constants::Codec::HEVC.repr(),
            None,
        )
        .unwrap();

    let config = loader.new_config().unwrap();
    // Set required API version to 2.2
    config
        .set_filter_property_u32(
            "mfxImplDescription.ApiVersion.Version",
            (2u32 << 16) + 2,
            None,
        )
        .unwrap();

    let mut session = loader.new_session(0).unwrap();

    let mut buffer: Vec<u8> = vec![0; DEFAULT_BUFFER_SIZE];
    let mut bitstream = Bitstream::with_codec(&mut buffer, constants::Codec::HEVC);
    let free_buffer_len = (bitstream.len() - bitstream.size() as usize) as u64;
    let bytes_read = io::copy(
        &mut io::Read::take(&mut file, free_buffer_len),
        &mut bitstream,
    )
    .unwrap();
    assert_ne!(bytes_read, 0);

    let mut params = session
        .decode_header(&mut bitstream, constants::IoPattern::OUT_SYSTEM_MEMORY)
        .unwrap();

    let decoder = session.decoder(&mut params).unwrap();

    loop {
        let free_buffer_len = (bitstream.len() - bitstream.size() as usize) as u64;
        let bytes_read = io::copy(
            &mut io::Read::take(&mut file, free_buffer_len),
            &mut bitstream,
        )
        .unwrap();

        let mut frame = match decoder.decode(Some(&mut bitstream)) {
            Ok(frame) => frame,
            Err(e) if e == MfxStatus::MoreData => {
                break;
            },
            Err(e) => panic!("{:?}", e)
        };
        // wait for frame
        frame.synchronize().unwrap();
        let bytes = io::copy(&mut frame, &mut output).unwrap();
        assert_ne!(bytes, 0);


        if bytes_read == 0 && bytes == 0 {
            break;
        }
    }

    // Now the flush the decoder pass None to decode
    loop {
        let mut frame = match decoder.decode(None) {
            Ok(frame) => frame,
            Err(e) if e == MfxStatus::MoreData => {
                break;
            },
            Err(e) => panic!("{:?}", e)
        };
        // wait for frame
        frame.synchronize().unwrap();
        io::copy(&mut frame, &mut output).unwrap();
    }
}

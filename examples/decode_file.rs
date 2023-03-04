///! This example decodes an hevc encoded file (tests/frozen.hevc) and produces a raw YUV 4:2:0 8 bit file at /tmp/output.yuv
use std::io;

use intel_onevpl_sys::MfxStatus;
use onevpl::{self, constants, init, Bitstream, Loader};

const DEFAULT_BUFFER_SIZE: usize = 1024 * 1024 * 2; // 2MB

#[tokio::main(flavor = "current_thread")]
pub async fn main() {
    init().unwrap();

    // Open file to read from
    let mut file = std::fs::File::open("tests/frozen.hevc").unwrap();
    let mut output = std::fs::File::create("/tmp/output.yuv").unwrap();

    let mut loader = Loader::new().unwrap();

    let config = loader.new_config().unwrap();
    // Set software decoding
    config
        .set_filter_property_u32(
            "mfxImplDescription.Impl",
            constants::Impl::Software.repr(),
            None,
        )
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

        let mut frame = match decoder.decode(Some(&mut bitstream), None).await {
            Ok(frame) => frame,
            Err(e) if e == MfxStatus::MoreData => {
                break;
            }
            Err(e) => panic!("{:?}", e),
        };

        let bytes = io::copy(&mut frame, &mut output).unwrap();
        assert_ne!(bytes, 0);

        if bytes_read == 0 && bytes == 0 {
            break;
        }
    }

    // Now the flush the decoder pass None to decode
    // "The application must set bs to NULL to signal end of stream. The application may need to call this API function several times to drain any internally cached frames until the function returns MFX_ERR_MORE_DATA."
    loop {
        let mut frame = match decoder.decode(None, None).await {
            Ok(frame) => frame,
            Err(e) if e == MfxStatus::MoreData => {
                break;
            }
            Err(e) => panic!("{:?}", e),
        };

        io::copy(&mut frame, &mut output).unwrap();
    }
}

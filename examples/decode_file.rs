///! This example decodes an hevc encoded file (tests/frozen.hevc) and produces a raw YUV 4:2:0 8 bit file at /tmp/output.yuv
use std::{env, io, path::PathBuf};

use intel_onevpl_sys::MfxStatus;
use onevpl::{bitstream::Bitstream, constants, Loader};

const DEFAULT_BUFFER_SIZE: usize = 1024 * 1024 * 2; // 2MB

#[tokio::main(flavor = "current_thread")]
pub async fn main() {
    // Setup basic logger
    tracing_subscriber::fmt::init();

    // Open file to read from
    let mut file = std::fs::File::open("tests/frozen.hevc").unwrap();
    let mut output_path = PathBuf::from(env::temp_dir());
    output_path.push("output.yuv");
    let mut output = std::fs::File::create(output_path.as_path()).unwrap();

    let mut loader = Loader::new().unwrap();

    // Set software decoding
    loader.use_hardware(false);

    // Set decode HEVC
    loader
        .set_filter_property(
            "mfxImplDescription.mfxDecoderDescription.decoder.CodecID",
            constants::Codec::HEVC,
            None,
        )
        .unwrap();

    // Set required API version to 2.2
    loader
        .set_filter_property(
            "mfxImplDescription.ApiVersion.Version",
            constants::ApiVersion::new(2, 2),
            None,
        )
        .unwrap();

    let session = loader.new_session(0).unwrap();

    // Create a backing buffer that will contain the bitstream we are trying to decode
    let mut buffer: Vec<u8> = vec![0; DEFAULT_BUFFER_SIZE];
    let mut bitstream = Bitstream::with_codec(&mut buffer, constants::Codec::HEVC);

    // We get the size of the buffer and subtract the amount of the buffer that
    // is used to get how much free buffer is available. io::copy will fail if
    // it's only able to copy a portion of what you tell it to copy.
    let free_buffer_len = (bitstream.len() - bitstream.size() as usize) as u64;
    let bytes_read = io::copy(
        &mut io::Read::take(&mut file, free_buffer_len),
        &mut bitstream,
    )
    .unwrap();

    // If we read 0 bytes from the input file, something is obviously wrong
    assert_ne!(bytes_read, 0);

    // Get information about the bitstream we are about to decode
    let params = session
        .decode_header(&mut bitstream, constants::IoPattern::SYSTEM_MEMORY)
        .unwrap();

    let decoder = session.decoder(params).unwrap();

    loop {
        let frame = match decoder.decode(Some(&mut bitstream), None, None).await {
            Ok(frame) => Some(frame),
            Err(e) if e == MfxStatus::MoreData => {
                let free_buffer_len = (bitstream.len() - bitstream.size() as usize) as u64;
                let bytes_read = io::copy(
                    &mut io::Read::take(&mut file, free_buffer_len),
                    &mut bitstream,
                )
                .unwrap();

                if bytes_read == 0 {
                    break;
                }

                None
            }
            Err(e) => panic!("{:?}", e),
        };

        if let Some(mut frame) = frame {
            let bytes = io::copy(&mut frame, &mut output).unwrap();
            assert_ne!(bytes, 0);
        }

        // // I don't think we'll hit this line, but if we don't read anything and
        // // don't write anything we want to break out of the loop.
        // if bytes_read == 0 && bytes == 0 {
        //     break;
        // }
    }

    // Now the flush the decoder pass None to decode. "The application must set
    // bs to NULL to signal end of stream. The application may need to call this
    // API function several times to drain any internally cached frames until
    // the function returns MFX_ERR_MORE_DATA."
    loop {
        let mut frame = match decoder.decode(None, None, None).await {
            Ok(frame) => frame,
            Err(e) if e == MfxStatus::MoreData => {
                break;
            }
            Err(e) => panic!("{:?}", e),
        };

        io::copy(&mut frame, &mut output).unwrap();
    }

    println!("Decoded file was written to: {}", output_path.display());
}

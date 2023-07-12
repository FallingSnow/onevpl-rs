use std::{io, path::PathBuf, env};

use intel_onevpl_sys::MfxStatus;
use onevpl::{self, bitstream::Bitstream, constants, vpp::VppVideoParams, Loader};

const DEFAULT_BUFFER_SIZE: usize = 1024 * 1024 * 2; // 2MB

#[tokio::main(flavor = "current_thread")]
pub async fn main() {
    tracing_subscriber::fmt::init();

    // Open file to read from
    let mut file = std::fs::File::open("tests/frozen.h264").unwrap();
    let mut output_path = PathBuf::from(env::temp_dir());
    output_path.push("output.yuv");
    let mut output = std::fs::File::create(output_path.as_path()).unwrap();

    let mut loader = Loader::new().unwrap();

    // Method 1: Intel API handles hardware selection
    // Set hardware decoding
    loader.use_hardware(true);

    // Set decode HEVC
    loader
        .set_filter_property(
            "mfxImplDescription.mfxDecoderDescription.decoder.CodecID",
            constants::Codec::HEVC,
            None,
        )
        .unwrap();

    // Set required API version to 1.35
    // See MFX_LEGACY_VERSION for latest legacy version
    loader
        .set_filter_property(
            "mfxImplDescription.ApiVersion.Version",
            constants::ApiVersion::new(1, 35),
            None,
        )
        .unwrap();

    println!("Implementations:");
    loader.implementations().expect("Failed to get available implementations")
    .iter().for_each(|i| {
        println!("\t- Name: {}", i.name());
    });

    let session = loader.new_session(0).unwrap();

    let mut buffer: Vec<u8> = vec![0; DEFAULT_BUFFER_SIZE];
    let mut bitstream = Bitstream::with_codec(&mut buffer, constants::Codec::AVC);
    let free_buffer_len = (bitstream.len() - bitstream.size() as usize) as u64;
    let bytes_read = io::copy(
        &mut io::Read::take(&mut file, free_buffer_len),
        &mut bitstream,
    )
    .unwrap();
    assert_ne!(bytes_read, 0);

    let mfx_params = session
        .decode_header(&mut bitstream, constants::IoPattern::OUT_VIDEO_MEMORY)
        .unwrap();

    // Intel hardware will decode into a hardware color format like nv12 for 8 bit
    // content. We will use the hardware video processor to convert this to yuv420
    // format.
    let mut vpp_params = VppVideoParams::from(&mfx_params);
    vpp_params.set_io_pattern(constants::IoPattern::OUT_SYSTEM_MEMORY | constants::IoPattern::IN_VIDEO_MEMORY);
    vpp_params.set_out_fourcc(constants::FourCC::YV12);

    let decoder = session.decoder(mfx_params).unwrap();
    let vpp = session.video_processor(&mut vpp_params).unwrap();

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
            Err(e) if e == MfxStatus::VideoParamChanged => {
                let _params = decoder.params().unwrap();
                println!("Video decoding parameters changed");
                None
            }
            Err(e) => panic!("{:?}", e),
        };

        if let Some(mut frame) = frame {
            let mut yuv_frame = vpp.process(Some(&mut frame), None).await.unwrap();
            let bytes = io::copy(&mut yuv_frame, &mut output).unwrap();
            assert_ne!(bytes, 0);
        }
    }

    // Now the flush the decoder pass None to decode
    // "The application must set bs to NULL to signal end of stream. The application may need to call this API function several times to drain any internally cached frames until the function returns MFX_ERR_MORE_DATA."
    loop {
        let mut frame = match decoder.decode(None, None, None).await {
            Ok(frame) => frame,
            Err(e) if e == MfxStatus::MoreData => {
                break;
            }
            Err(e) => panic!("{:?}", e),
        };
        let mut yuv_frame = vpp.process(Some(&mut frame), None).await.unwrap();
        io::copy(&mut yuv_frame, &mut output).unwrap();
    }

    println!("Decoded file was written to: {}", output_path.display());
}

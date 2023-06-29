///! This example encodes a yuv file (tests/frozen180.yuv) and produces a HEVC YUV 4:2:0 8 bit file at /tmp/output.hevc
use std::{env, path::PathBuf, io};

use intel_onevpl_sys::MfxStatus;
use onevpl::{
    bitstream::Bitstream,
    constants::{self, IoPattern},
    encode::EncodeCtrl,
    frameallocator::FrameAllocator,
    Loader, MfxVideoParams,
};

use onevpl::{self, vpp::VppVideoParams};

const DEFAULT_BUFFER_SIZE: usize = 1024 * 1024 * 2; // 2MB

#[tokio::main]
pub async fn main() {
    // Setup basic logger
    tracing_subscriber::fmt::init();

    // Open file to read from
    let mut file = std::fs::File::open("tests/frozen.hevc").unwrap();
    let mut output_path = PathBuf::from(env::temp_dir());
    output_path.push("output.yuv");
    let mut output = std::fs::File::create(output_path.as_path()).unwrap();

    let codec = constants::Codec::HEVC;

    let mut loader = Loader::new().unwrap();

    // Set software decoding
    loader.use_hardware(true);

    // Set decode HEVC
    loader.require_decoder(codec);

    // Set required API version to 2.2
    loader.use_api_version(2, 2);

    let mut session = loader.new_session(0).unwrap();

    let mut frame_allocator = FrameAllocator::new();
    frame_allocator.set_alloc_callback(Box::new(|request, response| {
        dbg!(request.num_frame_min());
        MfxStatus::Unsupported
    }));
    frame_allocator.set_lock_callback(Box::new(|id, data| {
        dbg!("Lock callback called");
        MfxStatus::Unsupported
    }));
    frame_allocator.set_unlock_callback(Box::new(|id, data| {
        dbg!("UnLock callback called");
        MfxStatus::Unsupported
    }));
    session.set_allocator(frame_allocator).unwrap();

    let mut params = MfxVideoParams::default();

    let mut buffer: Vec<u8> = vec![0; DEFAULT_BUFFER_SIZE];
    let mut bitstream = Bitstream::with_codec(&mut buffer, codec);
    let free_buffer_len = (bitstream.len() - bitstream.size() as usize) as u64;
    let bytes_read = io::copy(
        &mut io::Read::take(&mut file, free_buffer_len),
        &mut bitstream,
    )
    .unwrap();
    assert_ne!(bytes_read, 0);

    let mfx_params = session
        .decode_header(&mut bitstream, constants::IoPattern::OUT_VIDEO_MEMORY)
        .expect("Failed to discover video parameters");

    let mut vpp_params = VppVideoParams::from(&mfx_params);
    vpp_params.set_io_pattern(
        constants::IoPattern::VIDEO_MEMORY,
    );
    vpp_params.set_out_fourcc(constants::FourCC::YV12);

    let decoder = session.decoder(mfx_params).expect("Unable to create decoder");
    let vpp = session.video_processor(&mut vpp_params).unwrap();

    loop {
        let frame = match decoder.decode(Some(&mut bitstream), None).await {
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
        let mut frame = match decoder.decode(None, None).await {
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

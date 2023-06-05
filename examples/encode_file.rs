///! This example encodes a yuv file (tests/frozen180.yuv) and produces a HEVC YUV 4:2:0 8 bit file at /tmp/output.hevc
use std::{io::{BufRead, ErrorKind}, path::PathBuf, env};

use intel_onevpl_sys::MfxStatus;
use onevpl::{
    bitstream::Bitstream,
    constants::{self, IoPattern},
    encode::EncodeCtrl,
    Loader, MfxVideoParams,
};

#[tokio::main]
pub async fn main() {
    // Setup basic logger
    tracing_subscriber::fmt::init();

    // Open file to read from
    let file = std::fs::File::open("tests/frozen180.yuv").unwrap();
    let mut reader = std::io::BufReader::with_capacity(122880, file);
    let mut output_path = PathBuf::from(env::temp_dir());
    output_path.push("output.hevc");
    let mut output = std::fs::File::create(output_path).unwrap();

    let width = 320;
    let height = 180;
    let target_kbps = 1000;
    let codec = constants::Codec::HEVC;

    let mut loader = Loader::new().unwrap();

    // Set software decoding
    loader
        .set_filter_property(
            "mfxImplDescription.Impl",
            constants::ImplementationType::SOFTWARE,
            None,
        )
        .unwrap();

    // Set decode HEVC
    loader
        .set_filter_property(
            "mfxImplDescription.mfxEncoderDescription.encoder.CodecID",
            codec,
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

    let mut params = MfxVideoParams::default();

    // Encoding config
    params.set_codec(codec);
    params.set_target_usage(constants::TargetUsage::Level4);
    params.set_rate_control_method(constants::RateControlMethod::VBR);
    params.set_target_kbps(target_kbps);
    // 24000/1001 = 23.976 fps
    params.set_framerate(24000, 1001);

    // Input frame config
    params.set_fourcc(constants::FourCC::IyuvOrI420);
    params.set_chroma_format(constants::ChromaFormat::YUV420);
    params.set_io_pattern(IoPattern::IN_SYSTEM_MEMORY);

    // We must know before hand the size of the frames we are giving to the encoder
    params.set_height(height);
    params.set_width(width);
    params.set_crop(width, height);

    // dbg!(params);

    let mut encoder = session.encoder(params).unwrap();

    // Get the configured params from the encoder
    let params = encoder.params().unwrap();

    // Create a backing buffer that will contain the bitstream of the encoded output
    let suggested_buffer_size = params.suggested_buffer_size();
    let mut buffer: Vec<u8> = vec![0; suggested_buffer_size];
    let mut bitstream = Bitstream::with_codec(&mut buffer, codec);

    loop {
        // Try to fill read buffer with data from the input file, if end of file just continue
        // Loop break will be handled by the not having enough data to read one frame
        if let Err(e) = reader.fill_buf() {
            if e.kind() != ErrorKind::UnexpectedEof {
                panic!("{:?}", e);
            }
        };

        // Gives you additional per frame encoder controls that we won't use in this example
        let mut ctrl = EncodeCtrl::new();

        // In this example we let the encoder handle the allocation of surfaces
        let mut frame_surface = encoder.get_surface().unwrap();

        // Read a frame's worth of data from reader into the allocated FrameSurface
        // If we need more data to read one frame, we can assume we are done
        if let Err(e) = frame_surface.read_raw_frame(&mut reader, constants::FourCC::IyuvOrI420) {
            match e {
                MfxStatus::MoreData => break,
                _ => panic!("{:?}", e),
            };
        };

        // Attempt to encode a frame. The encode method returns the number of bytes written to the bitstream. If more data
        let bytes_written = match encoder
            .encode(&mut ctrl, Some(frame_surface), &mut bitstream, None)
            .await
        {
            Ok(bytes) => bytes,
            Err(e) if e == MfxStatus::MoreData => 0,
            Err(e) => panic!("{:?}", e),
        };

        // If data was written to the bitstream we try to copy the bitstream data to our output file
        if bytes_written > 0 {
            let bitstream_size = bitstream.size();
            let bytes_copied = std::io::copy(&mut bitstream, &mut output).unwrap();
            assert_eq!(bitstream_size as u64, bytes_copied);
        }
    }

    println!("Flushing encoder");

    loop {
        let mut ctrl = EncodeCtrl::new();
        let bytes_written = match encoder.encode(&mut ctrl, None, &mut bitstream, None).await {
            Ok(bytes) => bytes,
            Err(e) if e == MfxStatus::MoreData => break,
            Err(e) => panic!("{:?}", e),
        };

        if bytes_written > 0 {
            let bitstream_size = bitstream.size();
            let bytes_copied = std::io::copy(&mut bitstream, &mut output).unwrap();
            assert_eq!(bitstream_size as u64, bytes_copied);
        }
    }
}

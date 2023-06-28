///! This example encodes a yuv file (tests/frozen180.yuv) and produces a HEVC YUV 4:2:0 8 bit file at /tmp/output.hevc
use std::{path::PathBuf, env};

use intel_onevpl_sys::MfxStatus;
use onevpl::{
    bitstream::Bitstream,
    constants::{self, IoPattern, FourCC},
    encode::EncodeCtrl,
    Loader, MfxVideoParams, vpp::VppVideoParams, utils::{hw_align_height, hw_align_width},
};

#[tokio::main]
pub async fn main() {
    // Setup basic logger
    tracing_subscriber::fmt::init();

    // Open file to read from
    let mut file = std::fs::File::open("tests/frozen180.bgra").unwrap();
    let mut output_path = PathBuf::from(env::temp_dir());
    output_path.push("output.hevc");
    let mut output = std::fs::File::create(output_path.as_path()).unwrap();
    let width = 320;
    let height = 180;
    let input_frame_struct = constants::PicStruct::Progressive;
    let hw_width = hw_align_width(width);
    let hw_height = hw_align_height(height, input_frame_struct);
    let target_kbps = 1000;
    let codec = constants::Codec::HEVC;

    let mut loader = Loader::new().unwrap();

    // Set hardware encoding
    loader.use_hardware(true);

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

    // See https://spec.oneapi.io/versions/latest/elements/oneVPL/source/appendix/VPL_apnds_a.html#specifying-configuration-parameters for the parameters you have to set.
    let mut mfx_params = MfxVideoParams::default();

    // Encoding config
    mfx_params.set_codec(codec);
    mfx_params.set_target_usage(constants::TargetUsage::Level4);
    mfx_params.set_rate_control_method(constants::RateControlMethod::CBR);
    mfx_params.set_target_kbps(target_kbps);
    // 24000/1001 = 23.976 fps
    mfx_params.set_framerate(24000, 1001);

    // Input frame config
    mfx_params.set_fourcc(FourCC::YUY2);
    mfx_params.set_chroma_format(constants::ChromaFormat::YUV422);
    mfx_params.set_io_pattern(IoPattern::IN_VIDEO_MEMORY);

    // We must know before hand the size of the frames we are giving to the encoder
    mfx_params.set_height(hw_height);
    mfx_params.set_width(hw_width);
    mfx_params.set_crop(width, height);

    let mut encoder = session.encoder(mfx_params).unwrap();

    // Get the configured params from the encoder
    let mfx_params = encoder.params().unwrap();

    // We need to use the VPP because when HW encoding only frames in HW formats are supported (Eg. YUV12 -> NV12)
    let mut vpp_params = VppVideoParams::default();
    vpp_params.set_io_pattern(IoPattern::IN_SYSTEM_MEMORY | IoPattern::OUT_VIDEO_MEMORY);

    vpp_params.set_in_fourcc(FourCC::Rgb4OrBgra);
    vpp_params.set_in_picstruct(input_frame_struct);
    vpp_params.set_in_height(hw_height);
    vpp_params.set_in_width(hw_width);
    vpp_params.set_in_crop(0, 0, width, height);
    vpp_params.set_in_framerate(24000, 1001);
    
    vpp_params.set_out_fourcc(FourCC::YUY2);
    vpp_params.set_out_picstruct(constants::PicStruct::Progressive);
    vpp_params.set_out_height(hw_height);
    vpp_params.set_out_width(hw_width);
    vpp_params.set_out_crop(0, 0, width, height);
    vpp_params.set_out_framerate(24000, 1001);

    let mut vpp = session.video_processor(&mut vpp_params).unwrap();

    // Create a backing buffer that will contain the bitstream of the encoded output
    let suggested_buffer_size = mfx_params.suggested_buffer_size();
    let mut buffer: Vec<u8> = vec![0; suggested_buffer_size];
    let mut bitstream = Bitstream::with_codec(&mut buffer, codec);

    loop {

        // Gives you additional per frame encoder controls that we won't use in this example
        let mut ctrl = EncodeCtrl::new();

        let mut frame_surface = vpp.get_surface_input().unwrap();
        if let Err(e) = frame_surface.read_raw_frame(&mut file, constants::FourCC::Rgb4OrBgra).await {
            match e {
                MfxStatus::MoreData => break,
                _ => panic!("{:?}", e),
            };
        };

        let vpp_frame = vpp.process(Some(&mut frame_surface), None).await.unwrap();

        let bytes_written = match encoder
            .encode(&mut ctrl, Some(vpp_frame), &mut bitstream, None)
            .await
        {
            Ok(bytes) => bytes,
            Err(e) if e == MfxStatus::MoreData => 0,
            Err(e) => panic!("{:?}", e),
        };

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

    println!("Encoded file was written to: {}", output_path.display());
}

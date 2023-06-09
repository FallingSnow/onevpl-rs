use std::{env, path::PathBuf};

use crate::{
    constants::{self, FourCC, IoPattern},
    utils::{hw_align_height, hw_align_width},
    vpp::VppVideoParams,
    Loader,
};
use intel_onevpl_sys::MfxStatus;

async fn vpp_file() -> PathBuf {
    // Open file to read from
    let mut input = std::fs::File::open("tests/frozen180.yuv").unwrap();

    // Create output file
    let mut output_path = PathBuf::from(env::temp_dir());
    output_path.push("output-nv12.yuv");
    let mut output = std::fs::File::create(output_path.as_path()).unwrap();

    // Define some input parameters
    let width = 320;
    let height = 180;
    let input_frame_struct = constants::PicStruct::Progressive;
    let hw_width = hw_align_width(width);
    let hw_height = hw_align_height(height, input_frame_struct);

    let mut loader = Loader::new().unwrap();
    loader.use_hardware(true);

    // Set required API version to 2.2
    loader
        .set_filter_property(
            "mfxImplDescription.ApiVersion.Version",
            constants::ApiVersion::new(2, 2),
            None,
        )
        .unwrap();

    let session = loader.new_session(0).unwrap();

    let mut vpp_params = VppVideoParams::default();
    vpp_params.set_io_pattern(IoPattern::VIDEO_MEMORY);

    vpp_params.set_in_fourcc(FourCC::IyuvOrI420);
    vpp_params.set_in_picstruct(input_frame_struct);
    vpp_params.set_in_chroma_format(constants::ChromaFormat::YUV420);
    vpp_params.set_in_height(hw_height);
    vpp_params.set_in_width(hw_width);
    vpp_params.set_in_crop(0, 0, width, height);
    vpp_params.set_in_framerate(24000, 1001);

    vpp_params.set_out_fourcc(FourCC::NV12);
    vpp_params.set_out_picstruct(constants::PicStruct::Progressive);
    vpp_params.set_out_chroma_format(constants::ChromaFormat::YUV420);
    vpp_params.set_out_height(hw_height);
    vpp_params.set_out_width(hw_width);
    vpp_params.set_out_crop(0, 0, width, height);
    vpp_params.set_out_framerate(24000, 1001);

    let mut vpp = session.video_processor(&mut vpp_params).unwrap();

    loop {
        // Gives you additional per frame encoder controls that we won't use in this example

        let mut frame_surface = vpp.get_surface_input().unwrap();
        if let Err(e) = frame_surface.read_raw_frame(&mut input, constants::FourCC::IyuvOrI420).await {
            match e {
                MfxStatus::MoreData => break,
                _ => panic!("{:?}", e),
            };
        };

        let mut vpp_frame = vpp.process(Some(&mut frame_surface), None).await.unwrap();

        let bytes_copied = std::io::copy(&mut vpp_frame, &mut output).unwrap();
        assert_ne!(bytes_copied, 0);
    }

    output_path
}

#[tokio::test]
async fn test_vpp_file() {
    use sha2::Digest;
    let output_path = vpp_file().await;
    let mut hasher = sha2::Sha256::new();
    let mut file = std::fs::File::open(output_path).unwrap();

    std::io::copy(&mut file, &mut hasher).unwrap();
    let hash_bytes = hasher.finalize();

    assert_eq!(
        hash_bytes[..],
        hex_literal::hex!("782135cbab50e1c38b2e308e856f2d98953c7db5ed18274b54095320f18d2a02")
    );
}

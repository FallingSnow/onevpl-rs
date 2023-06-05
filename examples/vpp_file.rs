use std::io::{BufRead, ErrorKind};

use intel_onevpl_sys::MfxStatus;
use onevpl::{
    constants::{self, FourCC, IoPattern},
    vpp::VppVideoParams,
    Loader, utils::{hw_align_width, hw_align_height},
};

#[tokio::main]
pub async fn main() {
    // Setup basic logger
    tracing_subscriber::fmt::init();

    // Open file to read from
    let file = std::fs::File::open("tests/frozen180.yuv").unwrap();
    let mut reader = std::io::BufReader::with_capacity(122880, file);
    let mut output = std::fs::File::create("/tmp/output-nv12.yuv").unwrap();
    let width = 320;
    let height = 180;
    let input_frame_struct = constants::PicStruct::Progressive;
    let hw_width = hw_align_width(width);
    let hw_height = hw_align_height(height, input_frame_struct);

    let mut loader = Loader::new().unwrap();

    // Set software decoding
    loader
        .set_filter_property(
            "mfxImplDescription.Impl",
            constants::Implementation::HARDWARE,
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
        // Try to fill out read buffer, if end of file then break
        if let Err(e) = reader.fill_buf() {
            if e.kind() != ErrorKind::UnexpectedEof {
                panic!("{:?}", e);
            }
        };

        // Gives you additional per frame encoder controls that we won't use in this example

        let mut frame_surface = vpp.get_surface_input().unwrap();
        if let Err(e) = frame_surface.read_raw_frame(&mut reader, constants::FourCC::IyuvOrI420) {
            match e {
                MfxStatus::MoreData => break,
                _ => panic!("{:?}", e),
            };
        };

        let mut vpp_frame = vpp.process(Some(&mut frame_surface), None).await.unwrap();

        let bytes_copied = std::io::copy(&mut vpp_frame, &mut output).unwrap();
        assert_ne!(bytes_copied, 0);
    }
}

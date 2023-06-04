use std::io::{BufRead, ErrorKind};

use intel_onevpl_sys::MfxStatus;
use onevpl::{
    constants::{self, FourCC, IoPattern},
    vpp::VppVideoParams,
    FrameReader, Loader,
};

#[tokio::main]
pub async fn main() {
    // Setup basic logger
    tracing_subscriber::fmt::init();

    // Open file to read from
    let file = std::fs::File::open("tests/frozen180.yuv").unwrap();
    let reader = std::io::BufReader::with_capacity(122880, file);
    let mut output = std::fs::File::create("/tmp/output-nv12.yuv").unwrap();
    let width = 320;
    let height = 180;
    let mut frame_reader = FrameReader::new(reader, width, height, constants::FourCC::IyuvOrI420);

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
    let in_height = vpp_params.set_in_height(height);
    let in_width = vpp_params.set_in_width(width);
    vpp_params.set_in_crop(0, 0, width, height);
    vpp_params.set_in_framerate(24000, 1001);
    vpp_params.set_in_picstruct(constants::PicStruct::Progressive);

    vpp_params.set_out_fourcc(FourCC::NV12);
    let out_height = vpp_params.set_out_height(height);
    let out_width = vpp_params.set_out_width(width);
    vpp_params.set_out_crop(0, 0, width, height);
    vpp_params.set_out_framerate(24000, 1001);
    vpp_params.set_out_picstruct(constants::PicStruct::Progressive);

    dbg!(in_height, in_width, out_height, out_width);

    let mut vpp = session.video_processor(&mut vpp_params).unwrap();

    loop {
        // Try to fill out read buffer, if end of file then break
        if let Err(e) = frame_reader.fill_buf() {
            if e.kind() != ErrorKind::UnexpectedEof {
                panic!("{:?}", e);
            }
        };

        // Gives you additional per frame encoder controls that we won't use in this example

        let mut frame_surface = vpp.get_surface_input().unwrap();
        if let Err(e) = frame_surface.read_one_frame(&mut frame_reader) {
            match e {
                MfxStatus::MoreData => break,
                _ => panic!("{:?}", e),
            };
        };

        let vpp_frame = vpp.process(Some(&mut frame_surface), None).await.unwrap();

        let bytes_copied = std::io::copy(&mut frame_surface, &mut output).unwrap();
    }
}

//! The Indeo 3 and Indeo 5 registry decoders report the size of the frame
//! they last returned (oxideav-core `Decoder::output_video_dimensions` /
//! `output_pixel_format`), including a size change from one stream to the
//! next. Frames are decoded in `receive_frame`, so nothing is known
//! before the first one.
//!
//! Both decoders emit `Yuv444P` (chroma replicated to luma resolution).

use oxideav_core::{CodecId, Decoder, Frame, Packet, PixelFormat, TimeBase};
use oxideav_indeo::{indeo3, indeo5};

/// The frames of a vendored `IV32` corpus, split by its index.
fn iv32_frames(bin: &[u8], idx: &str) -> Vec<Vec<u8>> {
    idx.lines()
        .skip(1)
        .map(|l| {
            let f: Vec<usize> = l.split(',').skip(1).map(|v| v.parse().unwrap()).collect();
            bin[f[0]..f[0] + f[1]].to_vec()
        })
        .collect()
}

/// Feeds every packet, receiving after each, and checks the report and
/// the planes of each returned frame against the packet's size.
fn check(dec: &mut dyn Decoder, packets: Vec<(Vec<u8>, (u32, u32))>) {
    let mut last: Option<(u32, u32)> = None;
    for (at, (data, (w, h))) in packets.into_iter().enumerate() {
        dec.send_packet(&Packet::new(0, TimeBase::new(1, 25), data))
            .expect("send");
        if last.is_some() {
            assert_eq!(
                dec.output_video_dimensions(),
                last,
                "frame {at}: before it is returned"
            );
        }
        let frame = match dec.receive_frame() {
            Ok(Frame::Video(frame)) => frame,
            other => panic!("frame {at}: {other:?}"),
        };
        assert_eq!(dec.output_video_dimensions(), Some((w, h)), "frame {at}");
        assert_eq!(
            dec.output_pixel_format(),
            Some(PixelFormat::Yuv444P),
            "frame {at}"
        );
        let planes = frame.image_planes();
        assert_eq!(planes.len(), 3, "frame {at}: planes");
        for (i, plane) in planes.iter().enumerate() {
            assert_eq!(plane.stride, w as usize, "frame {at}: plane {i} stride");
            assert_eq!(
                plane.data.len(),
                (w * h) as usize,
                "frame {at}: plane {i} rows"
            );
        }
        last = Some((w, h));
    }
}

#[test]
fn indeo3_reports_each_frames_size() {
    let small = iv32_frames(
        include_bytes!("data/iv32-160x120-all-intra/samples.bin"),
        include_str!("data/iv32-160x120-all-intra/samples-index.csv"),
    );
    let large = iv32_frames(
        include_bytes!("data/iv32-176x144-4frame-intra-period/samples.bin"),
        include_str!("data/iv32-176x144-4frame-intra-period/samples-index.csv"),
    );
    // Two 160×120 frames, then the 176×144 stream from its INTRA frame.
    let packets = small
        .into_iter()
        .take(2)
        .map(|f| (f, (160, 120)))
        .chain(large.into_iter().take(3).map(|f| (f, (176, 144))))
        .collect();
    let mut dec = indeo3::Indeo3RegistryDecoder::new(CodecId::new(indeo3::CODEC_ID_STR));
    check(&mut dec, packets);
}

#[test]
fn indeo5_reports_each_frames_size() {
    let packets = vec![
        (
            include_bytes!("data/intra-320x240-indeo5.iv50").to_vec(),
            (320, 240),
        ),
        (
            include_bytes!("data/intra-240x180-educ.iv50").to_vec(),
            (240, 180),
        ),
    ];
    let mut dec = indeo5::Indeo5RegistryDecoder::new(CodecId::new(indeo5::CODEC_ID_STR));
    check(&mut dec, packets);
}

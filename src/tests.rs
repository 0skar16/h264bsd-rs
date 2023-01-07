use std::{fs::{read, self}, time::Instant};

use av_codec::decoder::Decoder as AVDecoder;
use av_data::packet::Packet;

use crate::Decoder;
use image::{codecs::png::PngEncoder, ImageEncoder};
#[test]
pub fn decode_and_save() {

    let mut decoder = Decoder::new().unwrap();
    let pic = read("test.h264").unwrap();
    let mut pkt = Packet::new();
    pkt.data = pic;
    let start = Instant::now();
    decoder.send_packet(&pkt).unwrap();
    let end = Instant::now();
    print!("Decoding took: {}ms", (end-start).as_millis());
    let frame = decoder.receive_frame().unwrap();
    let r = frame.buf.as_slice_inner(0).unwrap();
    let g = frame.buf.as_slice_inner(1).unwrap();
    let b = frame.buf.as_slice_inner(2).unwrap();
    let a = frame.buf.as_slice_inner(3).unwrap();
    let mut data = vec![];
    let vid = frame.kind.get_video_info().unwrap();
    for i in 0..vid.width*vid.height {
        data.push(r[i]);
        data.push(g[i]);
        data.push(b[i]);
        data.push(a[i]);
    }
    let f = fs::File::create("test.png").unwrap();
    PngEncoder::new(f).write_image(&data, vid.width as u32, vid.height as u32, image::ColorType::Rgba8).unwrap();
}
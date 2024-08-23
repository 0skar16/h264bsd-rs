use std::{path::PathBuf, env};

use bindgen::{Builder, CargoCallbacks};
use cc::Build;

fn main() {
    let mut builder = Build::new();
    let build = builder
        .files(vec![
            "h264bsd/src/h264bsd_byte_stream.c",
            "h264bsd/src/h264bsd_decoder.c",
            "h264bsd/src/h264bsd_intra_prediction.c",
            "h264bsd/src/h264bsd_pic_order_cnt.c",
            "h264bsd/src/h264bsd_seq_param_set.c",
            "h264bsd/src/h264bsd_storage.c",
            "h264bsd/src/h264bsd_vlc.c",
            "h264bsd/src/h264bsd_cavlc.c",
            "h264bsd/src/h264bsd_dpb.c",
            "h264bsd/src/h264bsd_macroblock_layer.c",
            "h264bsd/src/h264bsd_pic_param_set.c",
            "h264bsd/src/h264bsd_slice_data.c",
            "h264bsd/src/h264bsd_stream.c",
            "h264bsd/src/h264bsd_vui.c",
            "h264bsd/src/h264bsd_conceal.c",
            "h264bsd/src/h264bsd_image.c",
            "h264bsd/src/h264bsd_nal_unit.c",
            "h264bsd/src/h264bsd_reconstruct.c",
            "h264bsd/src/h264bsd_slice_group_map.c",
            "h264bsd/src/h264bsd_transform.c",
            "h264bsd/src/h264bsd_deblocking.c",
            "h264bsd/src/h264bsd_inter_prediction.c",
            "h264bsd/src/h264bsd_neighbour.c",
            "h264bsd/src/h264bsd_sei.c",
            "h264bsd/src/h264bsd_slice_header.c",
            "h264bsd/src/h264bsd_util.c"
        ].iter())
        .include("h264bsd/src/")
        .flag("-D_ERROR_PRINT")
        //.flag("-D_DEBUG_PRINT")
        ;
    build.compile("h264bsd");
    println!("cargo:rerun-if-changed=data/wrapper.h");
    let bindings = Builder::default()
        .header("data/wrapper.h")
        .parse_callbacks(Box::new(CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
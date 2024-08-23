use std::{ffi::c_void, io::{Error, ErrorKind, Result}, mem::MaybeUninit, sync::Arc};

use av_codec::decoder::Decoder as AVDecoder;
use av_data::{frame::{ArcFrame, Frame, VideoInfo}, packet::Packet, pixel::formats::{RGB24, RGBA, YUV420}};
use h264bsd_sys::*;
pub use h264bsd_sys;

#[cfg(test)]
mod tests;
pub struct Decoder {
    pub internal: storage_t,
    pub current_image: Option<Image>,
    pub size: (u32, u32),
    pos: (u32, u32),
    crop_flag: u32,
    output_type: ImageOutput,
}
impl Decoder {
    pub fn new(output_type: ImageOutput) -> Result<Self> {
        let mut internal: storage_t = unsafe{ std::mem::zeroed() };
        let status = unsafe { h264bsdInit(&mut internal, 0) };
        if status > 0 {
            return Err(Error::new(ErrorKind::Other, "Couldn't initiate h264 decoder!"));
        }

        Ok(Self {
            internal,
            current_image: None,
            size: (0,0),
            pos: (0,0),
            crop_flag: 0,
            output_type,
        })
    }
    fn internal(&mut self) -> *mut storage_t {
        &mut self.internal
    }
    pub unsafe fn decode(&mut self, mut data: Vec<u8>) -> Result<()> {
        let mut data: &mut [u8] = &mut data;
        let mut pic_data = 0 as *mut u8;
        let mut pic_id = 0;
        let mut is_idr_pic = 0;
        let mut num_err_mbs = 0;
        let mut got_img = false;
        while data.len() > 0 {
            let mut read = 0;
            let status = H264bsdStatus::try_from(h264bsdDecode(self.internal(), data.as_mut_ptr(), data.len() as u32, 0, &mut read))?;
            match status {
                H264bsdStatus::PicRdy => {
                    got_img = true;
                    pic_data = h264bsdNextOutputPicture(self.internal(), &mut pic_id, &mut is_idr_pic, &mut num_err_mbs);
                },
                H264bsdStatus::Error => Err(Error::new(ErrorKind::Other, "H264 error occured"))?,
                H264bsdStatus::ParamSetError => Err(Error::new(ErrorKind::Other, "H264 param set error occured"))?,
                H264bsdStatus::Rdy => {},
                H264bsdStatus::MemAllocError => Err(Error::new(ErrorKind::Other, "H264 memory allocation error occured"))?,
                H264bsdStatus::HdrsRdy => {
                    h264bsdCroppingParams(self.internal(), &mut self.crop_flag, &mut self.pos.0, &mut self.size.0, &mut self.pos.1, &mut self.size.1);
                    if self.crop_flag != 0 {
                        self.size.0 = h264bsdPicWidth(self.internal()) * 16;
                        self.size.1 = h264bsdPicHeight(self.internal()) * 16;
                    }
                },
            }
            //len -= read as usize;
            if read > 0 {
                data = &mut data[read as usize..];
            }
        }
        if got_img {
            let len = (self.size.0 * self.size.1 * 3 / 2) as usize;
            let img = Image{
                width: self.size.0,
                height: self.size.1,
                data: Vec::from_raw_parts(pic_data, len, len).clone(),
            };
            self.current_image = Some(img);
        }
        Ok(())
        
    }
}
impl AVDecoder for Decoder {
    fn set_extradata(&mut self, _: &[u8]) {
        
    }

    fn send_packet(&mut self, pkt: &Packet) -> av_codec::error::Result<()> {
        unsafe { self.decode(pkt.data.clone()).map_err(|_| av_codec::error::Error::InvalidData)? };
        Ok(())
    }

    fn receive_frame(&mut self) -> av_codec::error::Result<ArcFrame> {
        if let Some(img) = &self.current_image {
            let video = VideoInfo::new(
                img.width as usize,
                img.height as usize,
                false,
                av_data::frame::FrameType::OTHER,
                Arc::new(match self.output_type {
                    ImageOutput::RGBA => *RGBA,
                    ImageOutput::YUV => *YUV420,
                    ImageOutput::RGB => *RGB24,
                }),
            );
            let mut f = Frame::new_default_frame(video, None);
            match self.output_type {
                ImageOutput::RGBA =>  {
                    let (r, g, b) = convert_to_rgb(img.width as usize, img.height as usize, &img.data);
                    let a = vec![0xff; (img.width * img.height) as usize];

                    f.buf.as_mut_slice_inner(0).unwrap().copy_from_slice(&r);
                    f.buf.as_mut_slice_inner(1).unwrap().copy_from_slice(&g);
                    f.buf.as_mut_slice_inner(2).unwrap().copy_from_slice(&b);
                    f.buf.as_mut_slice_inner(3).unwrap().copy_from_slice(&a);
                },
                ImageOutput::YUV => {
                    let wh = (img.width*img.height) as usize;
                    f.buf.as_mut_slice_inner(0).unwrap().copy_from_slice(&img.data[..wh]);
                    f.buf.as_mut_slice_inner(1).unwrap().copy_from_slice(&img.data[wh..wh+wh/2]);
                    f.buf.as_mut_slice_inner(2).unwrap().copy_from_slice(&img.data[wh+wh/2..wh*2]);
                }
                ImageOutput::RGB => {
                    let (r, g, b) = convert_to_rgb(img.width as usize, img.height as usize, &img.data);

                    f.buf.as_mut_slice_inner(0).unwrap().copy_from_slice(&r);
                    f.buf.as_mut_slice_inner(1).unwrap().copy_from_slice(&g);
                    f.buf.as_mut_slice_inner(2).unwrap().copy_from_slice(&b);
                },
            }
            
            Ok(Arc::new(f))
        }else{
            Err(av_codec::error::Error::MoreDataNeeded)
        }
    }

    fn configure(&mut self) -> av_codec::error::Result<()> {
        Ok(())
    }

    fn flush(&mut self) -> av_codec::error::Result<()> {
        Ok(())
    }
}

unsafe impl Send for Decoder {}
unsafe impl Sync for Decoder {}

impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe {
            for i in 0..MAX_NUM_SEQ_PARAM_SETS as usize {
                if !self.internal.sps[i].is_null() {
                    free!((*self.internal.sps[i]).offsetForRefFrame);
                    free!((*self.internal.sps[i]).vuiParameters);
                    free!(self.internal.sps[i]);
                }
            }
            for i in 0..MAX_NUM_PIC_PARAM_SETS as usize {
                if !self.internal.pps[i].is_null() {
                    free!((*self.internal.pps[i]).runLength);
                    free!((*self.internal.pps[i]).topLeft);
                    free!((*self.internal.pps[i]).bottomRight);
                    free!((*self.internal.pps[i]).sliceGroupId);
                    free!(self.internal.pps[i]);
                }
            }
            free!(self.internal.mbLayer);
            free!(self.internal.mb);
            free!(self.internal.sliceGroupMap);

            free!(self.internal.conversionBuffer);

            if !self.internal.dpb[0].buffer.is_null() {
                for _i in 0..self.internal.dpb[0].dpbSize as isize + 1 {
                    // Throws an invalid memory reference for some reason
                    //free!((*self.internal.dpb[0].buffer.offset(_i)).data);
                }
            }

            free!(self.internal.dpb[0].buffer);
            free!(self.internal.dpb[0].list);
            free!(self.internal.dpb[0].outBuf);
        }
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum H264bsdStatus {
    PicRdy = H264BSD_PIC_RDY,
    Error = H264BSD_ERROR,
    ParamSetError = H264BSD_PARAM_SET_ERROR,
    Rdy = H264BSD_RDY,
    MemAllocError = H264BSD_MEMALLOC_ERROR,
    HdrsRdy = H264BSD_HDRS_RDY,
}
impl TryFrom<u32> for H264bsdStatus {
    type Error = Error;

    fn try_from(value: u32) -> std::result::Result<Self, <H264bsdStatus as TryFrom<u32>>::Error> {
        match value {
            H264BSD_PIC_RDY => Ok(H264bsdStatus::PicRdy),
            H264BSD_ERROR => Ok(H264bsdStatus::Error),
            H264BSD_PARAM_SET_ERROR => Ok(H264bsdStatus::ParamSetError),
            H264BSD_RDY => Ok(H264bsdStatus::Rdy),
            H264BSD_MEMALLOC_ERROR => Ok(H264bsdStatus::MemAllocError),
            H264BSD_HDRS_RDY => Ok(H264bsdStatus::HdrsRdy),
            _ => Err(Error::new(ErrorKind::Other, "Wrong status!")),
        }
    }

}
#[derive(Debug, Clone)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Copy)]
pub enum ImageOutput {
    RGBA,
    RGB,
    YUV,
}


pub fn convert_to_rgb(w: usize, h: usize, data: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let mut x = 0;
    let mut y = 0;
    let y_size = w * h;
    let cb_size = w / 2 * h / 2;
    let luma = &data[..y_size];
    let cb = &data[y_size..y_size + cb_size];
    let cr = &data[y_size + cb_size..];

    let mut r = vec![0u8; w * h];
    let mut g = vec![0u8; w * h];
    let mut b = vec![0u8; w * h];

    while y < h {
        let c = luma[x + y * w] as i32 - 16;
        let d = cb[x / 2 + y / 2 * w / 2] as i32 - 128;
        let e = cr[x / 2 + y / 2 * w / 2] as i32 - 128;
        r[x + y * w] = if (298 * c + 409 * e + 128 >> 8) < 0 {
            0
        } else if 298 * c + 409 * e + 128 >> 8 > 255 {
            255
        } else {
            (298 * c + 409 * e + 128 >> 8) as u32
        } as u8;
        g[x + y * w] = if (298 * c - 100 * d - 208 * e + 128 >> 8) < 0 {
            0
        } else if 298 * c - 100 * d - 208 * e + 128 >> 8 > 255 {
            255
        } else {
            298 * c - 100 * d - 208 * e + 128 >> 8
        } as u8;
        b[x + y * w] = if (298 * c + 516 * d + 128 >> 8) < 0 {
            0
        } else if 298 * c + 516 * d + 128 >> 8 > 255 {
            255
        } else {
            298 * c + 516 * d + 128 >> 8
        } as u8;

        x += 1;

        if x < w {
            continue;
        }

        x = 0;

        y += 1;
    }
    (r, g, b)
}
#[macro_export]
macro_rules! free {
    ($ptr:expr) => {
        if !$ptr.is_null() {
            libc::free($ptr as *mut c_void)
        }
    };
}
use std::{io::Result, io::Error, io::ErrorKind, sync::Arc, time::Instant};

use av_codec::decoder::Decoder as AVDecoder;
use av_data::{packet::Packet, frame::{ArcFrame, Frame, VideoInfo}, pixel::formats::RGBA};
use h264bsd_sys::{storage_t, h264bsdAlloc, h264bsdInit, h264bsdDecode, H264BSD_PIC_RDY, H264BSD_ERROR, H264BSD_PARAM_SET_ERROR, H264BSD_RDY, H264BSD_HDRS_RDY, H264BSD_MEMALLOC_ERROR, h264bsdCroppingParams, h264bsdPicWidth, h264bsdPicHeight, h264bsdNextOutputPicture, h264bsdConvertToRGBA, h264bsdShutdown, h264bsdFree};
#[cfg(test)]
mod tests;
pub struct Decoder {
    pub internal: *mut storage_t,
    current_image: Option<Image>,
}
impl Decoder {
    pub fn new() -> Result<Self> {
        let internal = unsafe{ h264bsdAlloc() };
        let status = unsafe { h264bsdInit(internal, 0) };
        if status > 0 {
            return Err(Error::new(ErrorKind::Other, "Couldn't initiate h264 decoder!"));
        }

        Ok(Self {
            internal,
            current_image: None,
        })
    }
    pub unsafe fn decode(&mut self, data: Vec<u8>) -> Result<Image> {
        let mut data = data;
        let mut top = 0;
        let mut left = 0;
        let mut width = 0;
        let mut height = 0;
        let mut cropping_flag = 0;
        let mut pic_data = vec![].as_mut_ptr();
        let mut pic_id = 0;
        let mut is_idr_pic = 0;
        let mut num_err_mbs = 0;
        while data.len() > 0 {
            let mut read = 0;
            let status = H264bsdStatus::try_from(h264bsdDecode(self.internal, data.as_mut_ptr(), data.len() as u32, 0, &mut read))?;
            match status {
                H264bsdStatus::PicRdy => {
                    pic_data = h264bsdNextOutputPicture(self.internal, &mut pic_id, &mut is_idr_pic, &mut num_err_mbs);
                },
                H264bsdStatus::Error => Err(Error::new(ErrorKind::Other, "H264 error occured"))?,
                H264bsdStatus::ParamSetError => Err(Error::new(ErrorKind::Other, "H264 param set error occured"))?,
                H264bsdStatus::Rdy => {},
                H264bsdStatus::MemAllocError => Err(Error::new(ErrorKind::Other, "H264 memory allocation error occured"))?,
                H264bsdStatus::HdrsRdy => {
                    h264bsdCroppingParams(self.internal, &mut cropping_flag, &mut left, &mut width, &mut top, &mut height);
                    if cropping_flag != 0 {
                        width = h264bsdPicWidth(self.internal) * 16;
                        height = h264bsdPicHeight(self.internal) * 16;
                    }
                },
            }
            //len -= read as usize;
            if read > 0 {
                data = (&mut data[read as usize..]).to_vec();
            }
        }
        let img = Image{
            width,
            height,
            data: pic_data,
        };
        self.current_image = Some(img.clone());
        Ok(img)
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
        let start = Instant::now();
        if let Some(img) = self.current_image.clone() {
            let video = VideoInfo::new(
                img.width as usize,
                img.height as usize,
                false,
                av_data::frame::FrameType::OTHER,
                Arc::new(*RGBA),
            );
            let mut f = Frame::new_default_frame(video, None);
            let mut rgba: Vec<u32> = vec![0; img.width as usize * img.height as usize];
            unsafe { h264bsdConvertToRGBA(img.width, img.height, img.data, rgba.as_mut_ptr()) };
            let mut planes = [vec![], vec![], vec![], vec![]];
            for pix in rgba {
                let p = pix.to_le_bytes();
                planes[0].push(p[0]);
                planes[1].push(p[1]);
                planes[2].push(p[2]);
                planes[3].push(p[3]);
            }
            f.buf.as_mut_slice_inner(0).unwrap().copy_from_slice(&planes[0]);
            f.buf.as_mut_slice_inner(1).unwrap().copy_from_slice(&planes[1]);
            f.buf.as_mut_slice_inner(2).unwrap().copy_from_slice(&planes[2]);
            f.buf.as_mut_slice_inner(3).unwrap().copy_from_slice(&planes[3]);
            let end = Instant::now();
            println!("Conversion took: {}ms",(end-start).as_millis());
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
        unsafe {h264bsdShutdown(self.internal);
        h264bsdFree(self.internal);}
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
#[derive(Clone)]
pub struct Image {
    width: u32,
    height: u32,
    data: *mut u8,
}
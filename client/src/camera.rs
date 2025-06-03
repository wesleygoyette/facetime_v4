use core::error::Error;
use opencv::{
    core::{Mat, MatTraitConst},
    videoio::{CAP_ANY, VideoCapture, VideoCaptureTrait, VideoCaptureTraitConst},
};

use crate::ascii_converter::AsciiConverter;

pub struct Camera {
    cam: VideoCapture,
    ascii_converter: AsciiConverter,
    frame: Mat,
}

impl Camera {
    pub fn new(width: i32, height: i32) -> Result<Self, Box<dyn Error>> {
        let cam = VideoCapture::new(0, CAP_ANY)?;

        if !cam.is_opened()? {
            return Err("Error: Could not open camera".into());
        }

        let ascii_converter = AsciiConverter::new(width, height);

        println!("Starting camera ASCII feed... Press Ctrl+C to exit");
        println!("Camera initialized successfully!");

        let frame = Mat::default();

        return Ok(Self {
            cam,
            ascii_converter,
            frame,
        });
    }

    pub fn get_frame(&mut self) -> Result<String, Box<dyn Error>> {
        self.cam.read(&mut self.frame)?;

        if self.frame.empty() {
            eprintln!("Warning: Empty frame captured");
        }

        let ascii_frame = self.ascii_converter.frame_to_ascii(&self.frame)?;

        return Ok(ascii_frame);
    }
}

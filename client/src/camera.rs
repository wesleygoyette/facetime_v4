use core::error::Error;
use opencv::{
    core::{Mat, MatTraitConst},
    videoio::{CAP_ANY, VideoCapture, VideoCaptureTrait, VideoCaptureTraitConst},
};

use clap::ValueEnum;

use crate::ascii_converter::{ASCII_CHARS, AsciiConverter};

pub trait Camera {
    fn get_frame(&mut self) -> Result<String, Box<dyn Error>>;
}

pub struct RealCamera {
    cam: VideoCapture,
    ascii_converter: AsciiConverter,
    frame: Mat,
}

pub struct TestCamera {
    width: i32,
    height: i32,
    test_pattern: TestPatten,
    frame_count: i32,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum TestPatten {
    BrokenOldTv,
    HeartRateMoniter,
    PoopPov,
}

impl RealCamera {
    pub fn new(width: i32, height: i32) -> Result<Self, Box<dyn Error>> {
        let cam: VideoCapture = VideoCapture::new(0, CAP_ANY)?;

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
}

impl Camera for RealCamera {
    fn get_frame(&mut self) -> Result<String, Box<dyn Error>> {
        self.cam.read(&mut self.frame)?;

        if self.frame.empty() {
            return Err("Empty frame captured".into());
        }

        let ascii_frame = self.ascii_converter.frame_to_ascii(&self.frame)?;

        return Ok(ascii_frame);
    }
}

impl TestCamera {
    pub fn new(width: i32, height: i32, test_pattern: TestPatten) -> Result<Self, Box<dyn Error>> {
        return Ok(Self {
            width,
            height,
            test_pattern,
            frame_count: 0,
        });
    }
}

impl Camera for TestCamera {
    fn get_frame(&mut self) -> Result<String, Box<dyn Error>> {
        self.frame_count += 1;

        let time = self.frame_count / 4;

        let frame = match self.test_pattern {
            TestPatten::PoopPov => {
                let mut output = String::new();
                for y in 0..self.height {
                    for x in 0..self.width {
                        let index = ((x + y + time) % ASCII_CHARS.len() as i32) as usize;
                        output.push(ASCII_CHARS[index]);
                    }
                    output.push('\n');
                }
                output
            }
            TestPatten::HeartRateMoniter => {
                let mut output = String::new();
                let cx = self.width as f32 / 2.0;
                let cy = self.height as f32 / 2.0;
                for y in 0..self.height {
                    for x in 0..self.width {
                        let dx = x as f32 - cx;
                        let dy = y as f32 - cy;
                        let dist = ((dx * dx + dy * dy).sqrt() / 2.5) as i32;
                        let index = ((dist + time) % ASCII_CHARS.len() as i32) as usize;
                        output.push(ASCII_CHARS[index]);
                    }
                    output.push('\n');
                }
                output
            }
            TestPatten::BrokenOldTv => {
                let mut output = String::new();
                for y in 0..self.height {
                    for x in 0..self.width {
                        let wave = ((x as f32 / 5.0 + time as f32 / 5.0).sin() * 5.0
                            + (self.height / 2) as f32) as i32;
                        if wave == y {
                            output.push('@');
                        } else {
                            output.push(ASCII_CHARS[0]);
                        }
                    }
                    output.push('\n');
                }
                output
            }
        };

        return Ok(frame);
    }
}

use std::ops::Mul;

use crate::ascii_converter::{ASCII_CHARS, AsciiConverter};
use clap::ValueEnum;
use opencv::{
    core::{Mat, MatTraitConst},
    videoio::{CAP_ANY, VideoCapture, VideoCaptureTrait, VideoCaptureTraitConst},
};

pub trait Camera: Send {
    fn get_frame(&mut self) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;
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
    #[value(name = "lines")]
    BrokenOldTv,

    #[value(name = "waves")]
    HeartRateMoniter,

    #[value(name = "circle")]
    PoopPov,
}

impl RealCamera {
    pub fn new(width: i32, height: i32) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let cam: VideoCapture = VideoCapture::new(0, CAP_ANY)?;

        if !cam.is_opened()? {
            return Err("Error: Could not open camera".into());
        }

        let ascii_converter = AsciiConverter::new(width, height);

        let frame = Mat::default();

        return Ok(Self {
            cam,
            ascii_converter,
            frame,
        });
    }
}

impl Camera for RealCamera {
    fn get_frame(&mut self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        self.cam.read(&mut self.frame)?;

        if self.frame.empty() {
            return Err("Empty frame captured".into());
        }

        let ascii_frame = self.ascii_converter.frame_to_ascii(&self.frame)?;

        return Ok(ascii_frame);
    }
}

impl TestCamera {
    pub fn new(
        width: i32,
        height: i32,
        test_pattern: TestPatten,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        return Ok(Self {
            width,
            height,
            test_pattern,
            frame_count: 0,
        });
    }
}

impl Camera for TestCamera {
    fn get_frame(&mut self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        self.frame_count += 1;

        let time = self.frame_count;

        let frame = match self.test_pattern {
            TestPatten::BrokenOldTv => {
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
            TestPatten::PoopPov => {
                let mut output = String::new();
                let cx = self.width as f32 / 2.0;
                let cy = self.height as f32 / 2.0;
                for y in 0..self.height {
                    for x in 0..self.width {
                        let dx = x as f32 - cx;
                        let dy = (y as f32 - cy).mul(2 as f32);
                        let dist = ((dx * dx + dy * dy).sqrt() / 2.5) as i32;
                        let index = ((dist + time) % ASCII_CHARS.len() as i32) as usize;
                        output.push(ASCII_CHARS[index]);
                    }
                    output.push('\n');
                }
                output
            }
            TestPatten::HeartRateMoniter => {
                let mut output = String::new();

                let thickness = 1;

                for y in 0..self.height {
                    for x in 0..self.width {
                        let height_f = self.height as f32;

                        let amp1 = height_f * 0.2;
                        let amp2 = height_f * 0.15;
                        let amp3 = height_f * 0.1;

                        let wave1_y = ((height_f / 2.0)
                            + (x as f32 / 12.0 + time as f32 / 5.0).sin() * amp1)
                            as i32;
                        let wave2_y = ((height_f / 2.0)
                            + (x as f32 / 20.0 + time as f32 / 6.5).cos() * amp2)
                            as i32;
                        let wave3_y = ((height_f / 2.0)
                            + (x as f32 / 10.0 + time as f32 / 3.0).sin() * amp3)
                            as i32;

                        let current_y = y as i32;

                        let mut count = 0;
                        if (current_y - wave1_y).abs() <= thickness {
                            count += 1;
                        }
                        if (current_y - wave2_y).abs() <= thickness {
                            count += 1;
                        }
                        if (current_y - wave3_y).abs() <= thickness {
                            count += 1;
                        }

                        let ch = match count {
                            3 => '@',
                            2 => '#',
                            1 => '+',
                            _ => ' ',
                        };

                        output.push(ch);
                    }
                    output.push('\n');
                }

                output
            }
        };

        return Ok(frame);
    }
}

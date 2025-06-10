use std::ops::Mul;

use tokio::time::{Duration, Instant, sleep};

use crate::ascii_converter::ASCII_CHARS;
use clap::ValueEnum;
use opencv::{
    core::{CV_8UC1, Mat, MatExprTraitConst, MatTrait, MatTraitConst},
    videoio::{CAP_ANY, VideoCapture, VideoCaptureTrait, VideoCaptureTraitConst},
};

pub const MAX_FRAME_RATE: u64 = 120;

pub enum CameraKind {
    Real(RealCamera),
    Test(TestCamera),
}

impl CameraKind {
    pub async fn get_frame(&mut self) -> Result<&Mat, Box<dyn std::error::Error + Send + Sync>> {
        match self {
            CameraKind::Real(cam) => cam.get_frame().await,
            CameraKind::Test(cam) => cam.get_frame().await,
        }
    }
}

pub struct RealCamera {
    cam: VideoCapture,
    frame: Mat,
}

pub struct TestCamera {
    width: i32,
    height: i32,
    test_pattern: TestPatten,
    frame_count: i32,
    frame: Mat,
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
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let cam: VideoCapture = VideoCapture::new(0, CAP_ANY)?;

        if !cam.is_opened()? {
            return Err("Error: Could not open camera".into());
        }

        let frame = Mat::default();
        Ok(Self { cam, frame })
    }

    pub async fn get_frame(&mut self) -> Result<&Mat, Box<dyn std::error::Error + Send + Sync>> {
        let start_time = Instant::now();

        self.cam.read(&mut self.frame)?;

        if self.frame.empty() {
            return Err("Empty frame captured".into());
        }

        let target_frame_duration = Duration::from_millis(1000 / MAX_FRAME_RATE);
        let elapsed = start_time.elapsed();

        if elapsed < target_frame_duration {
            sleep(target_frame_duration - elapsed).await;
        }

        Ok(&self.frame)
    }
}

impl TestCamera {
    pub fn new(
        width: i32,
        height: i32,
        test_pattern: TestPatten,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let frame = Mat::zeros(height, width, CV_8UC1)?.to_mat()?;
        Ok(Self {
            width,
            height,
            test_pattern,
            frame_count: 0,
            frame,
        })
    }

    pub async fn get_frame(&mut self) -> Result<&Mat, Box<dyn std::error::Error + Send + Sync>> {
        let start_time = Instant::now();
        self.frame_count += 1;
        let time = self.frame_count * 20 / MAX_FRAME_RATE as i32;

        let mut output = Mat::zeros(self.height, self.width, CV_8UC1)?.to_mat()?;

        match self.test_pattern {
            TestPatten::BrokenOldTv => {
                for y in 0..self.height {
                    for x in 0..self.width {
                        let index = ((x + y + time) % ASCII_CHARS.len() as i32) as usize;
                        let val = (index as f32 / ASCII_CHARS.len() as f32 * 255.0) as u8;
                        *output.at_2d_mut::<u8>(y, x)? = val;
                    }
                }
            }
            TestPatten::PoopPov => {
                let cx = self.width as f32 / 2.0;
                let cy = self.height as f32 / 2.0;
                for y in 0..self.height {
                    for x in 0..self.width {
                        let dx = x as f32 - cx;
                        let dy = (y as f32 - cy).mul(2.0);
                        let dist = ((dx * dx + dy * dy).sqrt() / 2.5) as i32;
                        let index = ((dist + time) % ASCII_CHARS.len() as i32) as usize;
                        let val = (index as f32 / ASCII_CHARS.len() as f32 * 255.0) as u8;
                        *output.at_2d_mut::<u8>(y, x)? = val;
                    }
                }
            }
            TestPatten::HeartRateMoniter => {
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

                        let val = match count {
                            3 => 255,
                            2 => 180,
                            1 => 100,
                            _ => 0,
                        };

                        *output.at_2d_mut::<u8>(y, x)? = val;
                    }
                }
            }
        }

        let target_frame_duration = Duration::from_millis(1000 / MAX_FRAME_RATE);
        let elapsed = start_time.elapsed();

        if elapsed < target_frame_duration {
            sleep(target_frame_duration - elapsed).await;
        }

        self.frame = output;
        Ok(&self.frame)
    }
}

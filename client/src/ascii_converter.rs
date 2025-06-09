use core::error::Error;
use std::io::{Write, stdout};

use opencv::{
    core::{AlgorithmHint, BORDER_DEFAULT, CV_8U, CV_32F, Mat, Point_, Size, add_weighted},
    imgproc::{
        COLOR_BGR2GRAY, INTER_LINEAR, canny, create_clahe, cvt_color, filter_2d, gaussian_blur,
        resize,
    },
    prelude::*,
};

pub const ASCII_CHARS: &[char] = &[
    ' ', '.', '^', '=', '~', '-', ',', ':', ';', '+', '*', '?', '%', 'S', '#', '@',
];

pub const WIDTH: i32 = 92;
pub const HEIGHT: i32 = 28;

pub struct AsciiConverter {
    last_frame: Option<String>,
    terminal_size: Option<(u16, u16)>,
}

impl AsciiConverter {
    pub fn new() -> Self {
        Self {
            last_frame: None,
            terminal_size: None,
        }
    }

    pub fn frame_to_nibbles(frame: &Mat) -> Result<Vec<u8>, Box<dyn Error>> {
        let frame = enhance_frame(frame)?;

        let mut resized = Mat::default();
        let size = Size::new(WIDTH, HEIGHT);
        resize(&frame, &mut resized, size, 0.0, 0.0, INTER_LINEAR)?;

        let data = resized.data_bytes()?;
        let mut nibbles = Vec::with_capacity((WIDTH * HEIGHT / 2) as usize);

        for row in 0..HEIGHT {
            let row_start = (row * WIDTH) as usize;

            for col in (0..WIDTH).step_by(2) {
                let x1 = WIDTH - 1 - col;
                let x2 = if col + 1 < WIDTH {
                    WIDTH - 1 - (col + 1)
                } else {
                    0
                };

                let p1 = data[row_start + x1 as usize];
                let nibble1 = ((p1 as u16 * 15) / 255) as u8;

                let nibble2 = if col + 1 < WIDTH {
                    let p2 = data[row_start + x2 as usize];
                    ((p2 as u16 * 15) / 255) as u8
                } else {
                    0
                };

                nibbles.push((nibble1 << 4) | nibble2);
            }
        }

        Ok(nibbles)
    }

    pub fn nibbles_to_ascii(nibbles: &[u8], width: u16, height: u16) -> String {
        let mut grayscale: Vec<u8> = Vec::with_capacity((WIDTH as usize) * (HEIGHT as usize));
        for byte in nibbles {
            let high = (byte >> 4) & 0x0F;
            let low = byte & 0x0F;
            grayscale.push(high);
            grayscale.push(low);
        }

        let mut ascii_art = String::with_capacity((width + 1) as usize * height as usize);
        for y in 0..height {
            let src_y = (y as f32 * (HEIGHT as f32 - 1.0) / (height as f32)).round() as i32;
            let sy = (src_y.max(0).min(HEIGHT - 1)) as usize;

            for x in 0..width {
                let src_x = (x as f32 * (WIDTH as f32) / (width as f32)).round() as i32;
                let sx = (src_x.max(0).min(WIDTH - 1)) as usize;

                let idx = sy * WIDTH as usize + sx;
                let pixel = if idx < grayscale.len() {
                    grayscale[idx]
                } else {
                    0
                };

                let ascii_idx = (pixel as usize * (ASCII_CHARS.len() - 1)) / 15;
                ascii_art.push(ASCII_CHARS[ascii_idx]);
            }

            ascii_art.push('\n');
        }

        ascii_art
    }

    pub fn update_terminal_smooth(
        &mut self,
        new_content: &str,
        terminal_width: u16,
        terminal_height: u16,
    ) -> Result<(), Box<dyn Error>> {
        let size_changed = self
            .terminal_size
            .map(|(w, h)| w != terminal_width || h != terminal_height)
            .unwrap_or(true);

        if size_changed {
            Self::clear_terminal();
            self.terminal_size = Some((terminal_width, terminal_height));
            self.last_frame = None;
        }

        if let Some(ref last) = self.last_frame {
            if self.try_differential_update(last, new_content)? {
                self.last_frame = Some(new_content.to_string());
                return Ok(());
            }
        }

        print!("\x1B[1;1H");
        print!("{}", new_content);

        if let Some(ref last) = self.last_frame {
            let new_lines = new_content.lines().count();
            let old_lines = last.lines().count();

            if old_lines > new_lines {
                for _ in new_lines..old_lines {
                    print!("\x1B[K\n");
                }
            }
        }

        stdout().flush()?;
        self.last_frame = Some(new_content.to_string());
        Ok(())
    }

    fn try_differential_update(
        &self,
        old_content: &str,
        new_content: &str,
    ) -> Result<bool, Box<dyn Error>> {
        let old_lines: Vec<&str> = old_content.lines().collect();
        let new_lines: Vec<&str> = new_content.lines().collect();

        if old_lines.len().abs_diff(new_lines.len()) > 5 {
            return Ok(false);
        }

        let mut updated = false;
        let max_lines = old_lines.len().max(new_lines.len());

        for (line_num, (old_line, new_line)) in old_lines
            .iter()
            .zip(new_lines.iter())
            .enumerate()
            .take(max_lines)
        {
            if old_line != new_line {
                print!("\x1B[{};1H", line_num + 1);
                print!("\x1B[K");
                print!("{}", new_line);
                updated = true;
            }
        }

        if old_lines.len() > new_lines.len() {
            for line_num in new_lines.len()..old_lines.len() {
                print!("\x1B[{};1H", line_num + 1);
                print!("\x1B[K");
            }
            updated = true;
        }

        if updated {
            stdout().flush()?;
        }

        Ok(true)
    }

    pub fn clear_terminal() {
        print!("\x1B[2J\x1B[1;1H");
        stdout().flush().unwrap();
    }
}

fn enhance_frame(frame: &Mat) -> Result<Mat, Box<dyn Error>> {
    let mut gray = Mat::default();
    if frame.channels() != 1 {
        cvt_color(
            frame,
            &mut gray,
            COLOR_BGR2GRAY,
            0,
            AlgorithmHint::ALGO_HINT_DEFAULT,
        )?;
    } else {
        gray = frame.clone();
    }

    let mut clahe = create_clahe(1.5, Size::new(8, 8))?;
    let mut contrast = Mat::default();
    clahe.apply(&gray, &mut contrast)?;

    let mut blurred = Mat::default();
    gaussian_blur(
        &contrast,
        &mut blurred,
        Size::new(3, 3),
        0.0,
        0.0,
        BORDER_DEFAULT,
        AlgorithmHint::ALGO_HINT_DEFAULT,
    )?;

    let kernel_data = [0.0f32, -0.5, 0.0, -0.5, 3.0, -0.5, 0.0, -0.5, 0.0];
    let kernel = Mat::from_slice_2d(&[&kernel_data[0..3], &kernel_data[3..6], &kernel_data[6..9]])?;

    let mut sharpened = Mat::default();
    filter_2d(
        &blurred,
        &mut sharpened,
        -1,
        &kernel,
        Point_::new(-1, -1),
        0.0,
        BORDER_DEFAULT,
    )?;

    let mut edges = Mat::default();
    canny(&sharpened, &mut edges, 100.0, 200.0, 3, false)?;

    let mut edges_f32 = Mat::default();
    edges.convert_to(&mut edges_f32, CV_32F, 1.0 / 255.0, 0.0)?;

    let mut sharpened_f32 = Mat::default();
    sharpened.convert_to(&mut sharpened_f32, CV_32F, 1.0 / 255.0, 0.0)?;

    let mut final_enhanced = Mat::default();
    add_weighted(
        &sharpened_f32,
        1.0,
        &edges_f32,
        0.15,
        0.0,
        &mut final_enhanced,
        -1,
    )?;

    let mut output = Mat::default();
    final_enhanced.convert_to(&mut output, CV_8U, 255.0, 0.0)?;

    Ok(output)
}

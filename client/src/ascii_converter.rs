use core::error::Error;

use opencv::{
    core::{AlgorithmHint, Mat, Size},
    imgproc::{COLOR_BGR2GRAY, INTER_LINEAR, cvt_color, resize},
    prelude::*,
};

pub const ASCII_CHARS: &[char] = &[' ', '.', ',', ':', ';', '+', '*', '?', '%', 'S', '#', '@'];

pub const WIDTH: i32 = 90;
pub const HEIGHT: i32 = 28;

pub struct AsciiConverter {}

impl AsciiConverter {
    pub fn frame_to_nibbles(frame: &Mat) -> Result<Vec<u8>, Box<dyn Error>> {
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

        let mut resized = Mat::default();
        let size = Size::new(WIDTH, HEIGHT);
        resize(&gray, &mut resized, size, 0.0, 0.0, INTER_LINEAR)?;

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
            let src_y = y as f32 * (HEIGHT as f32) / (height as f32);
            let sy = src_y.floor() as usize;

            for x in 0..width {
                let src_x = x as f32 * (WIDTH as f32) / (width as f32);
                let sx = src_x.floor() as usize;

                let idx = sy * WIDTH as usize + sx;
                let pixel = if idx < grayscale.len() {
                    grayscale[idx]
                } else {
                    0
                };

                let ascii_idx = (pixel as usize).min(15).min(ASCII_CHARS.len() - 1);
                ascii_art.push(ASCII_CHARS[ascii_idx]);
            }

            ascii_art.push('\n');
        }

        ascii_art
    }

    pub fn clear_terminal() {
        print!("\x1B[2J\x1B[1;1H");
        use std::io::{Write, stdout};
        stdout().flush().unwrap();
    }
}

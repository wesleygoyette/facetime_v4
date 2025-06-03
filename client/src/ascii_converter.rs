use std::collections::HashMap;

use opencv::{
    core::{AlgorithmHint, Mat, Size},
    imgproc::{COLOR_BGR2GRAY, INTER_LINEAR, cvt_color, resize},
    prelude::*,
};

pub const ASCII_CHARS: &[char] = &[' ', '.', ',', ':', ';', '+', '*', '?', '%', 'S', '#', '@'];

pub struct AsciiConverter {
    width: i32,
    height: i32,
}

impl AsciiConverter {
    pub fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }

    pub fn frame_to_ascii(&self, frame: &Mat) -> opencv::Result<String> {
        let mut gray = Mat::default();
        cvt_color(
            frame,
            &mut gray,
            COLOR_BGR2GRAY,
            0,
            AlgorithmHint::ALGO_HINT_DEFAULT,
        )?;

        let mut resized = Mat::default();
        let size = Size::new(self.width, self.height);
        resize(&gray, &mut resized, size, 0.0, 0.0, INTER_LINEAR)?;

        let mut ascii_art = String::new();

        for y in 0..self.height {
            for x in 0..self.width {
                let pixel_value = *resized.at_2d::<u8>(y, self.width - 1 - x)?;
                let ascii_index = (pixel_value as usize * (ASCII_CHARS.len() - 1)) / 255;
                ascii_art.push(ASCII_CHARS[ascii_index]);
            }
            ascii_art.push('\n');
        }

        Ok(ascii_art)
    }

    pub fn ascii_frame_to_bytes(ascii: String) -> Vec<u8> {
        let mut all_chars = ASCII_CHARS.to_vec();
        all_chars.push('\n');

        let char_to_code: HashMap<char, u8> = all_chars
            .iter()
            .enumerate()
            .map(|(i, &ch)| (ch, i as u8))
            .collect();

        let mut bytes = Vec::with_capacity((ascii.len() + 1) / 2);

        let mut chars = ascii.chars();
        while let Some(ch1) = chars.next() {
            let code1 = *char_to_code.get(&ch1).unwrap_or(&0) & 0x0F;
            let ch2 = chars.next();
            let code2 = ch2
                .map(|c| *char_to_code.get(&c).unwrap_or(&0) & 0x0F)
                .unwrap_or(0);

            let byte = (code1 << 4) | code2;
            bytes.push(byte);
        }

        bytes
    }

    pub fn bytes_to_ascii_frame(bytes: &[u8]) -> String {
        let mut all_chars = ASCII_CHARS.to_vec();
        all_chars.push('\n');

        let mut ascii = String::with_capacity(bytes.len() * 2);

        for &byte in bytes {
            let high_nibble = (byte >> 4) & 0x0F;
            let low_nibble = byte & 0x0F;

            ascii.push(all_chars.get(high_nibble as usize).copied().unwrap_or(' '));
            ascii.push(all_chars.get(low_nibble as usize).copied().unwrap_or(' '));
        }

        ascii
    }

    pub fn clear_terminal() {
        print!("\x1B[2J\x1B[1;1H");
        use std::io::{Write, stdout};
        stdout().flush().unwrap();
    }
}

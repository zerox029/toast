use core::fmt;
use core::fmt::Arguments;
use conquer_once::spin::OnceCell;
use limine::framebuffer::Framebuffer;
use lazy_static::lazy_static;
use spin::Mutex;
use crate::{FRAMEBUFFER_REQUEST, serial_println};
use crate::utils::fonts::{FONT, FONT_HEIGHT, FONT_WIDTH};

// TODO: These shouldn't be constants, find a way to get the values from Limine while still allowing for a buffer
pub const SCREEN_WIDTH: usize = 1280;
pub const SCREEN_HEIGHT: usize = 800;
pub const BUFFER_WIDTH: usize = SCREEN_WIDTH / FONT_WIDTH;
pub const BUFFER_HEIGHT: usize = SCREEN_HEIGHT / FONT_HEIGHT;

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
    });
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Rgb8(pub u32);

impl Rgb8 {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Rgb8(r as u32 & 0xFF0000 | g as u32 & 0xFF00 | b as u32 & 0xFF)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ColorCode {
    foreground: Rgb8,
    background: Rgb8,
}

impl ColorCode {
    pub fn new(foreground: Rgb8, background: Rgb8) -> Self {
        Self { foreground, background }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

impl ScreenChar {
    pub fn new(ascii_character: u8, color_code: ColorCode) -> Self {
        Self { ascii_character, color_code }
    }
}

pub struct Writer {
    column_position: usize,
}

impl Writer {
    pub fn write_char(&mut self, screen_char: ScreenChar) {
        match screen_char.ascii_character {
            b'\n' => self.new_line(),
            _ => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                draw_char(screen_char, col, row);

                self.column_position += FONT_WIDTH;
            }
        }
    }

    pub fn new_line(&mut self) {
        if let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response() {
            if let Some(framebuffer) = framebuffer_response.framebuffers().next() {
                for y in (0..framebuffer.height() as usize).rev() {
                    for x in (0..framebuffer.width() as usize).rev() {

                        let pixel_offset = y * framebuffer.pitch() as usize + x * 4;
                        let pixel_value = unsafe { *(framebuffer.addr().add(pixel_offset) as *mut u32) };

                        display_pixel(&framebuffer, x, y + FONT_HEIGHT, pixel_value);
                    }
                }

                self.column_position = 0;
            }
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_char(ScreenChar::new(byte, ColorCode::new(Rgb8(0xFFFFFF), Rgb8(0))));
        }
        Ok(())
    }
}

fn draw_char(screen_char: ScreenChar, column: usize, row: usize) {
    if let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response() {
        if let Some(framebuffer) = framebuffer_response.framebuffers().next() {
            let mask = [128, 64, 32, 16, 8, 4, 2, 1];
            let glyph = FONT[screen_char.ascii_character as usize];

            for cy in 0..FONT_HEIGHT {
                for cx in 0..FONT_WIDTH {
                    let color = if glyph[cy] & mask[cx] == 0 {
                        screen_char.color_code.background
                    } else {
                        screen_char.color_code.foreground
                    };

                    display_pixel(&framebuffer, cx + column * FONT_WIDTH, cy + row * FONT_HEIGHT, color.0)
                }
            }
        }
    }
}

pub fn display_pixel(framebuffer: &Framebuffer, x: usize, y: usize, color: u32) {
    let pixel_offset = y * framebuffer.pitch() as usize + x * 4;
    unsafe { *(framebuffer.addr().add(pixel_offset) as *mut u32) = color; };
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::framebuffer::print(format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! println {
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

pub fn print(args: Arguments) {
    use core::fmt::Write;

    WRITER.lock().write_fmt(args).unwrap();
}
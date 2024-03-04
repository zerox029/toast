use alloc::{format, vec};
use alloc::vec::Vec;
use core::arch::asm;
use core::fmt;
use core::fmt::{Arguments, Write};
use core::mem::size_of;
use conquer_once::spin::OnceCell;
use limine::framebuffer::Framebuffer;
use spin::Mutex;
use crate::{FRAMEBUFFER_REQUEST, serial_println};
use crate::graphics::fonts::{FONT, FONT_HEIGHT, FONT_WIDTH};

pub static INSTANCE: OnceCell<Mutex<Writer>> = OnceCell::uninit();

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
    buffer_width: usize,
    buffer_height: usize,
    column_position: usize,
    screen_buffer: Vec<Vec<Option<ScreenChar>>>,

}

impl Writer {
    pub fn instance() -> Option<&'static Mutex<Writer>> {
        INSTANCE.get()
    }

    /// This function is unsafe because it should only be called once the heap is set up
    pub unsafe fn init() {
        let framebuffer = FRAMEBUFFER_REQUEST
            .get_response().expect("could not retrieve the framebuffer")
            .framebuffers().next().expect("could not retrieve the framebuffer");

        let buffer_width = framebuffer.width() as usize / FONT_WIDTH;
        let buffer_height = framebuffer.height() as usize / FONT_HEIGHT;

        let screen_buffer = vec![vec![None; buffer_height]; buffer_width];

        let writer = Self {
            buffer_width,
            buffer_height,
            column_position: 0,
            screen_buffer,
        };

        INSTANCE.init_once(|| Mutex::new(writer));
    }

    pub fn write_char(&mut self, screen_char: ScreenChar) {
        let row = self.buffer_height - 1;
        let col = self.column_position;

        self.column_position += 1;

        self.write_at(screen_char, col, row);
    }

    pub fn write_at(&mut self, screen_char: ScreenChar, col: usize, row: usize) {
        match screen_char.ascii_character {
            b'\n' => self.new_line(),
            _ => {
                if self.column_position >= self.buffer_width {
                    self.new_line();
                }

                self.screen_buffer[col][row] = Some(screen_char);

                draw_char(screen_char, col, row);
            }
        }
    }

    pub fn new_line(&mut self) {
        for row in (1..self.buffer_height) {
            for col in (0..self.buffer_width) {
                let character = self.screen_buffer[col][row];

                if let Some(character) = character {
                    self.write_at(character, col, row - 1);
                }
            }
        }

        self.clear_row(self.buffer_height - 1);
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: ColorCode::new(Rgb8(0xFFFFFF), Rgb8(0)),
        };

        for col in 0..self.buffer_width {
            self.write_at(blank, col, row);
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

pub fn display_pixel(framebuffer: &Framebuffer, col: usize, row: usize, color: u32) {
    let pixel_offset = row * framebuffer.pitch() as usize + col * 4;
    unsafe { *(framebuffer.addr().add(pixel_offset) as *mut u32) = color; };
}

macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::graphics::framebuffer::_print(format_args!($($arg)*));
    });
}

macro_rules! println {
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

#[doc(hidden)]
pub fn _print(args: Arguments) {
    use core::fmt::Write;

    let writer = Writer::instance();
    match writer {
        Some(writer) => {
            writer.lock().write_fmt(args).unwrap()
        }
        None => {
            serial_println!("buffer uninitialized");
        }
    }
}
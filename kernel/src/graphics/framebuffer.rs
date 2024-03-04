use alloc::{vec};
use alloc::vec::Vec;
use core::fmt::Write;
use conquer_once::spin::OnceCell;
use limine::framebuffer::Framebuffer;
use spin::Mutex;
use crate::{FRAMEBUFFER_REQUEST, serial_println};
use crate::graphics::fonts::{FONT, FONT_HEIGHT, FONT_WIDTH};

const DEFAULT_COLOR_CODE: ColorCode = ColorCode::new(Rgb8(0xFFFFFF), Rgb8(0));

pub static INSTANCE: OnceCell<Mutex<Writer>> = OnceCell::uninit();

pub enum LogLevel {
    Info,
    Warning,
    Error,
    Ok,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct Rgb8(pub u32);

impl Rgb8 {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Rgb8(r as u32 & 0xFF0000 | g as u32 & 0xFF00 | b as u32 & 0xFF)
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct ColorCode {
    foreground: Rgb8,
    background: Rgb8,
}

impl ColorCode {
    pub const fn new(foreground: Rgb8, background: Rgb8) -> Self {
        Self { foreground, background }
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
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
    color_code: ColorCode,
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
            color_code: DEFAULT_COLOR_CODE,
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

                draw_char(screen_char, col, row);
                self.screen_buffer[col][row] = Some(screen_char);
            }
        }
    }

    pub fn clear_at(&mut self, col: usize, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: ColorCode::new(Rgb8(0xFFFFFF), Rgb8(0)),
        };

        draw_char(blank, col, row);
        self.screen_buffer[col][row] = None;
    }

    fn clear_row(&mut self, row: usize) {
        for col in 0..self.buffer_width {
            self.clear_at(col, row);
        }
    }

    pub fn new_line(&mut self) {
        for row in (1..self.buffer_height) {
            for col in (0..self.buffer_width) {
                let bottom_character = self.screen_buffer[col][row];
                let top_character = self.screen_buffer[col][row - 1];

                if top_character != bottom_character {
                    if let Some(character) = bottom_character {
                        self.write_at(character, col, row - 1);
                    }
                    else {
                        self.clear_at(col, row - 1);
                    }
                }
            }
        }

        self.clear_row(self.buffer_height - 1);
        self.column_position = 0;
    }
}

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            self.write_char(ScreenChar::new(byte, self.color_code));
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

#[macro_export]
macro_rules! info {
    ($fmt:expr) => ({
        $crate::graphics::framebuffer::_print_header($crate::graphics::framebuffer::LogLevel::Info);
        print!(concat!($fmt, "\n"));
    });
    ($fmt:expr, $($arg:tt)*) => ({
        $crate::graphics::framebuffer::_print_header($crate::graphics::framebuffer::LogLevel::Info);
        print!(concat!($fmt, "\n"), $($arg)*);
    });
}

#[macro_export]
macro_rules! warn {
    ($fmt:expr) => ({
        $crate::graphics::framebuffer::_print_header($crate::graphics::framebuffer::LogLevel::Warning);
        print!(concat!($fmt, "\n"));
    });
    ($fmt:expr, $($arg:tt)*) => ({
        $crate::graphics::framebuffer::_print_header($crate::graphics::framebuffer::LogLevel::Warning);
        print!(concat!($fmt, "\n"), $($arg)*);
    });
}

#[macro_export]
macro_rules! error {
    ($fmt:expr) => ({
        $crate::graphics::framebuffer::_print_header($crate::graphics::framebuffer::LogLevel::Error);
        print!(concat!($fmt, "\n"));
    });
    ($fmt:expr, $($arg:tt)*) => ({
        $crate::graphics::framebuffer::_print_header($crate::graphics::framebuffer::LogLevel::Error);
        print!(concat!($fmt, "\n"), $($arg)*);
    });
}

#[macro_export]
macro_rules! ok {
    ($fmt:expr) => ({
        $crate::graphics::framebuffer::_print_header($crate::graphics::framebuffer::LogLevel::Ok);
        print!(concat!($fmt, "\n"));
    });
    ($fmt:expr, $($arg:tt)*) => ({
        $crate::graphics::framebuffer::_print_header($crate::graphics::framebuffer::LogLevel::Ok);
        print!(concat!($fmt, "\n"), $($arg)*);
    });
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
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

#[doc(hidden)]
pub fn _print_header(header_type: LogLevel) {
    let mut writer = Writer::instance();

    match writer {
        Some(writer) => {
            let mut writer = writer.lock();

            match header_type {
                LogLevel::Info => {
                    writer.write_str("[ ").unwrap();

                    writer.color_code = ColorCode::new(Rgb8(0x5b616b), Rgb8(0));
                    writer.write_str("INFO").unwrap();
                    writer.color_code = DEFAULT_COLOR_CODE;

                    writer.write_str(" ] ").unwrap();
                }
                LogLevel::Warning => {
                    writer.write_str("[ ").unwrap();

                    writer.color_code = ColorCode::new(Rgb8(0xFFFF00), Rgb8(0));
                    writer.write_str("WARN").unwrap();
                    writer.color_code = DEFAULT_COLOR_CODE;

                    writer.write_str(" ] ").unwrap();
                }
                LogLevel::Error => {
                    writer.write_str("[ ").unwrap();

                    writer.color_code = ColorCode::new(Rgb8(0xFF4100), Rgb8(0));
                    writer.write_str("FAIL").unwrap();
                    writer.color_code = DEFAULT_COLOR_CODE;

                    writer.write_str(" ] ").unwrap();
                }
                LogLevel::Ok => {
                    writer.write_str("[ ").unwrap();

                    writer.color_code = ColorCode::new(Rgb8(0x00FF00), Rgb8(0));
                    writer.write_str(" OK ").unwrap();
                    writer.color_code = DEFAULT_COLOR_CODE;

                    writer.write_str(" ] ").unwrap();
                }
            }
        },
        None => {
            serial_println!("buffer uninitialized");
        }
    }
}
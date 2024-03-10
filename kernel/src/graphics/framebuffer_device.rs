use alloc::{vec};
use alloc::vec::Vec;
use core::fmt::Write;
use conquer_once::spin::OnceCell;
use rlibc::memcpy;
use spin::Mutex;
use crate::{FRAMEBUFFER_REQUEST, serial_println};
use crate::drivers::fbdev::FB_DEVICES;
use crate::fs::{VfsNode};
use crate::graphics::fonts::{FONT, FONT_HEIGHT, FONT_WIDTH};
use crate::serial::serial_print;

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
        if FB_DEVICES.lock().len() <= 0 {
            panic!("no framebuffer found");
        }

        let framebuffer = &FB_DEVICES.lock()[0];

        let buffer_width = framebuffer.screen_info.width as usize / FONT_WIDTH;
        let buffer_height = framebuffer.screen_info.height as usize / FONT_HEIGHT;

        let screen_buffer = vec![vec![None; buffer_width]; buffer_height];

        let writer = Self {
            color_code: DEFAULT_COLOR_CODE,
            buffer_width,
            buffer_height,
            column_position: 0,
            screen_buffer,
        };

        INSTANCE.try_init_once(|| Mutex::new(writer)).expect("Cannot initialize the framebuffer more than once");
    }

    fn write_char(&mut self, screen_char: ScreenChar) {
        let row = self.buffer_height - 1;
        let col = self.column_position;

        self.column_position += 1;

        self.write_at(screen_char, col, row);
    }

    fn write_at(&mut self, screen_char: ScreenChar, col: usize, row: usize) {
        match screen_char.ascii_character {
            b'\n' => self.new_line(),
            _ => {
                if self.column_position >= self.buffer_width {
                    self.new_line();
                }

                draw_char(screen_char, col, row);
                self.screen_buffer[row][col] = Some(screen_char);
            }
        }
    }

    fn clear_char(&mut self) {
        let row = self.buffer_height - 1;
        let col = self.column_position - 1;

        self.column_position -= 1;

        self.clear_at(col, row);
    }

    fn clear_at(&mut self, col: usize, row: usize) {
        if let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response() {
            if let Some(framebuffer) = framebuffer_response.framebuffers().next() {
                let empty_row = vec![0; FONT_WIDTH];

                for pixel_row in 0..FONT_HEIGHT {
                    let pixel_offset = ((row * FONT_HEIGHT) + pixel_row) * framebuffer.pitch() as usize + (col * FONT_WIDTH * 4);
                    unsafe { memcpy(framebuffer.addr().add(pixel_offset), empty_row.as_ptr() as *const u8, empty_row.len() * 4); }
                }

                self.screen_buffer[row][col] = None;
            }
        }
    }

    fn clear_row(&mut self, row: usize) {
        if let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response() {
            if let Some(framebuffer) = framebuffer_response.framebuffers().next() {
                let empty_row = vec![0; self.buffer_width * FONT_WIDTH];

                for pixel_row in 0..FONT_HEIGHT {
                    let pixel_offset = ((row * FONT_HEIGHT) + pixel_row) * framebuffer.pitch() as usize;
                    unsafe { memcpy(framebuffer.addr().add(pixel_offset), empty_row.as_ptr() as *const u8, empty_row.len() * 4); }
                }

                for col in 0..self.buffer_width {
                    self.screen_buffer[row][col] = None;
                }
            }
        }
    }

    fn new_line(&mut self) {
        for row in 1..self.buffer_height {
            for col in 0..self.buffer_width {
                let bottom_character = self.screen_buffer[row][col];
                let top_character = self.screen_buffer[row - 1][col];

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

impl Write for Writer {
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

            for (cy, glyph) in glyph.iter().enumerate().take(FONT_HEIGHT) {
                let mut scanrow: [u32; FONT_WIDTH] = [0; FONT_WIDTH];
                for (cx, mask) in mask.iter().enumerate().take(FONT_WIDTH) {
                    let color = if glyph & mask == 0 {
                        screen_char.color_code.background
                    } else {
                        screen_char.color_code.foreground
                    };

                    scanrow[cx] = color.0;
                }

                let c = column * FONT_WIDTH;
                let r = cy + row * FONT_HEIGHT;
                let pixel_offset = r * framebuffer.pitch() as usize + c * 4;
                FB_DEVICES.lock()[0].write(scanrow.as_ptr() as *const u8, scanrow.len() * 4, pixel_offset)
            }
        }
    }
}

pub fn backspace() {
    let writer = Writer::instance();
    match writer {
        Some(writer) => {
            writer.lock().clear_char();
        }
        None => {
            serial_println!("buffer uninitialized");
        }
    }
}

macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::graphics::framebuffer_device::_print(format_args!($($arg)*));
    });
}

macro_rules! println {
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

#[macro_export]
macro_rules! info {
    ($fmt:expr) => ({
        $crate::graphics::framebuffer_device::_print_header($crate::graphics::framebuffer_device::LogLevel::Info);
        print!(concat!($fmt, "\n"));
    });
    ($fmt:expr, $($arg:tt)*) => ({
        $crate::graphics::framebuffer_device::_print_header($crate::graphics::framebuffer_device::LogLevel::Info);
        print!(concat!($fmt, "\n"), $($arg)*);
    });
}

#[macro_export]
macro_rules! warn {
    ($fmt:expr) => ({
        $crate::graphics::framebuffer_device::_print_header($crate::graphics::framebuffer_device::LogLevel::Warning);
        print!(concat!($fmt, "\n"));
    });
    ($fmt:expr, $($arg:tt)*) => ({
        $crate::graphics::framebuffer_device::_print_header($crate::graphics::framebuffer_device::LogLevel::Warning);
        print!(concat!($fmt, "\n"), $($arg)*);
    });
}

#[macro_export]
macro_rules! error {
    ($fmt:expr) => ({
        $crate::graphics::framebuffer_device::_print_header($crate::graphics::framebuffer_device::LogLevel::Error);
        print!(concat!($fmt, "\n"));
    });
    ($fmt:expr, $($arg:tt)*) => ({
        $crate::graphics::framebuffer_device::_print_header($crate::graphics::framebuffer_device::LogLevel::Error);
        print!(concat!($fmt, "\n"), $($arg)*);
    });
}

#[macro_export]
macro_rules! ok {
    ($fmt:expr) => ({
        $crate::graphics::framebuffer_device::_print_header($crate::graphics::framebuffer_device::LogLevel::Ok);
        print!(concat!($fmt, "\n"));
    });
    ($fmt:expr, $($arg:tt)*) => ({
        $crate::graphics::framebuffer_device::_print_header($crate::graphics::framebuffer_device::LogLevel::Ok);
        print!(concat!($fmt, "\n"), $($arg)*);
    });
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    let writer = Writer::instance();
    match writer {
        Some(writer) => {
            writer.lock().write_fmt(args).unwrap()
        }
        None => {
            serial_print(args);
        }
    }
}

#[doc(hidden)]
pub fn _print_header(header_type: LogLevel) {
    let writer = Writer::instance();

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
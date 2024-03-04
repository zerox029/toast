use core::fmt;
use core::ptr::Unique;
use spin::Mutex;
use volatile::Volatile;

pub const VGA_BUFFER_ADDRESS: usize = 0xb8000;
const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

const DEFAULT_COLOR_CODE: ColorCode = ColorCode::new(Color::White, Color::Black);

pub static WRITER: Mutex<Writer> = Mutex::new(Writer {
    column_position: 0,
    color_code: DEFAULT_COLOR_CODE,
    buffer: unsafe { Unique::new_unchecked(0xb8000 as *mut _) },
});

pub enum MessageType {
    Info,
    Warning,
    Error,
    Ok,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Color {
    Black      = 0,
    Blue       = 1,
    Green      = 2,
    Cyan       = 3,
    Red        = 4,
    Magenta    = 5,
    Brown      = 6,
    LightGray  = 7,
    DarkGray   = 8,
    LightBlue  = 9,
    LightGreen = 10,
    LightCyan  = 11,
    LightRed   = 12,
    Pink       = 13,
    Yellow     = 14,
    White      = 15,
}

#[derive(Debug, Clone, Copy)]
pub struct ColorCode(u8);

impl ColorCode {
    pub const fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

impl ScreenChar {
    pub fn new(ascii_character: u8, color_code: ColorCode) -> Self {
        Self { ascii_character, color_code }
    }
}

struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: Unique<Buffer>,
}

impl Writer {
    fn buffer(&mut self) -> &mut Buffer {
        unsafe{ self.buffer.as_mut() }
    }

    fn set_color_code(&mut self, color_code: ColorCode) {
        self.color_code = color_code;
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                let color_code = self.color_code;
                self.buffer().chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code,
                });
                self.column_position += 1;
            }
        }
    }

    pub fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte)
        }
    }

    fn new_line(&mut self) {
        /*
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let buffer = self.buffer();
                let character = buffer.chars[row][col].read();
                buffer.chars[row - 1][col].write(character);
            }
        }
        self.clear_row(BUFFER_HEIGHT-1);
        self.column_position = 0;*/
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer().chars[row][col].write(blank);
        }
    }

    fn clear_char(&mut self) {
        let row = BUFFER_HEIGHT - 1;
        let col = self.column_position - 1;
        let color_code = self.color_code;

        self.buffer().chars[row][col].write(ScreenChar {
            ascii_character: b' ',
            color_code,
        });

        self.column_position -= 1;
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte)
        }
        Ok(())
    }
}


#[macro_export]
macro_rules! vga_print {
    ($($arg:tt)*) => ({
        $crate::vga_buffer::print(format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! vga_println {
    ($fmt:expr) => (vga_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (vga_print!(concat!($fmt, "\n"), $($arg)*));
}

#[macro_export]
macro_rules! info {
    ($fmt:expr) => ({
        $crate::vga_buffer::print_header($crate::vga_buffer::MessageType::Info);
        vga_print!(concat!($fmt, "\n"));
    });
    ($fmt:expr, $($arg:tt)*) => ({
        $crate::vga_buffer::print_header($crate::vga_buffer::MessageType::Info);
        vga_print!(concat!($fmt, "\n"), $($arg)*);
    });
}

#[macro_export]
macro_rules! warn {
    ($fmt:expr) => ({
        $crate::vga_buffer::print_header($crate::vga_buffer::MessageType::Warning);
        vga_print!(concat!($fmt, "\n"));
    });
    ($fmt:expr, $($arg:tt)*) => ({
        $crate::vga_buffer::print_header($crate::vga_buffer::MessageType::Warning);
        vga_print!(concat!($fmt, "\n"), $($arg)*);
    });
}

#[macro_export]
macro_rules! error {
    ($fmt:expr) => ({
        $crate::vga_buffer::print_header($crate::vga_buffer::MessageType::Error);
        vga_print!(concat!($fmt, "\n"));
    });
    ($fmt:expr, $($arg:tt)*) => ({
        $crate::vga_buffer::print_header($crate::vga_buffer::MessageType::Error);
        vga_print!(concat!($fmt, "\n"), $($arg)*);
    });
}

#[macro_export]
macro_rules! ok {
    ($fmt:expr) => ({
        $crate::vga_buffer::print_header($crate::vga_buffer::MessageType::Ok);
        vga_print!(concat!($fmt, "\n"));
    });
    ($fmt:expr, $($arg:tt)*) => ({
        $crate::vga_buffer::print_header($crate::vga_buffer::MessageType::Ok);
        vga_print!(concat!($fmt, "\n"), $($arg)*);
    });
}

pub fn print(args: fmt::Arguments) {
    use core::fmt::Write;



    //WRITER.lock().write_fmt(args).unwrap();
}

pub fn print_header(header_type: MessageType) {
    let mut writer = WRITER.lock();

    match header_type {
        MessageType::Info => {
            writer.write_str("[ ");

            writer.color_code = ColorCode::new(Color::DarkGray, Color::Black);
            writer.write_str("INFO");
            writer.color_code = DEFAULT_COLOR_CODE;

            writer.write_str(" ] ");
        }
        MessageType::Warning => {
            writer.write_str("[ ");

            writer.color_code = ColorCode::new(Color::Yellow, Color::Black);
            writer.write_str("WARN");
            writer.color_code = DEFAULT_COLOR_CODE;

            writer.write_str(" ] ");
        }
        MessageType::Error => {
            writer.write_str("[ ");

            writer.color_code = ColorCode::new(Color::Red, Color::Black);
            writer.write_str("FAIL");
            writer.color_code = DEFAULT_COLOR_CODE;

            writer.write_str(" ] ");
        }
        MessageType::Ok => {
            writer.write_str("[ ");

            writer.color_code = ColorCode::new(Color::Green, Color::Black);
            writer.write_str(" OK ");
            writer.color_code = DEFAULT_COLOR_CODE;

            writer.write_str(" ] ");
        }
    }
}

pub fn clear_screen() {
    for _ in 0..BUFFER_HEIGHT {
        vga_println!("");
    }
}

pub fn backspace() {
    WRITER.lock().clear_char();
}
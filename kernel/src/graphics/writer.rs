use alloc::vec;
use alloc::vec::Vec;
use conquer_once::spin::OnceCell;
use spin::Mutex;
use crate::drivers::fbdev::FB_DEVICES;
use crate::fs::VfsNode;
use crate::graphics::fonts::{FONT, FONT_HEIGHT, FONT_WIDTH};
use crate::serial_println;

static WRITER: OnceCell<Mutex<FramebufferWriter>> = OnceCell::uninit();

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

pub struct FramebufferWriter {
    buffer_height: usize,
    buffer_width: usize,

    column_position: usize,

    back_buffer: Vec<Vec<u32>>,
}
impl FramebufferWriter {
    pub fn instance() -> Option<&'static Mutex<FramebufferWriter>> {
        WRITER.get()
    }

    /// This function is unsafe because it should only be called once the heap is set up
    pub unsafe fn init() -> Result<(), &'static str> {
        if FB_DEVICES.lock().len() <= 0 {
            return Err("no framebuffer found");
        }

        let framebuffer_device = &FB_DEVICES.lock()[0];

        let buffer_width = framebuffer_device.screen_info.width as usize;
        let buffer_height = framebuffer_device.screen_info.height as usize;

        serial_println!("fb0 {}x{}", buffer_width, buffer_height);

        let back_buffer = vec![vec![0; buffer_width]; buffer_height];

        let writer = Self {
            buffer_height,
            buffer_width,
            back_buffer,
            column_position: 0,
        };

        WRITER.try_init_once(|| Mutex::new(writer)).or(Err("Cannot initialize the writer more than once"))
    }

    fn write_char_at(&mut self, char: u8, col: usize, row: usize) {
        match char {
            b'\n' => { },
            _ => {
                if self.column_position >= self.buffer_width {
                    //self.new_line();
                }

                let mask = [128, 64, 32, 16, 8, 4, 2, 1];
                let glyph = FONT[char as usize];
                for (cy, glyph) in glyph.iter().enumerate().take(FONT_HEIGHT) {
                    for (cx, mask) in mask.iter().enumerate().take(FONT_WIDTH) {
                        let color = if glyph & mask == 0 {
                            0
                        } else {
                            0xFFFFFF
                        };

                        self.back_buffer[cx + col][cy + row] = color;
                    }
                }

                self.swap_buffers();
            }
        }
    }

    fn swap_buffers(&self) {
        let framebuffer_device = &FB_DEVICES.lock()[0];
        framebuffer_device.write(self.back_buffer.as_ptr() as *const u8, self.buffer_width * self.buffer_height, 0);
    }
}
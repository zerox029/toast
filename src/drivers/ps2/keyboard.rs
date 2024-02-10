use crate::drivers::ps2::{DATA_PORT, PS2Device, PS2DeviceType, PS2Port};
use crate::drivers::ps2::PS2DeviceType::MF2Keyboard;
use crate::{println, print, vga_buffer};

#[repr(u8)]
enum Command {
    SetLEDs = 0xED,
    Echo = 0xEE,
    GetSetCurrentScancodeSet = 0xF0,
    Identify = 0xF2,
    SetTypematicRateAndDelay = 0xF3,
    EnableScanning = 0xF4,
    DisableScanning = 0xF5,
    SetDefaultParameters = 0xF6,
    SetAllTypematicAutorepeat = 0xF7,
    SetAllMakeRelease = 0xF8,
    SetAllMakeOnly = 0xF9,
    SetTypematicAutorepeatMakeRelease = 0xFA,
    SetSpecificTypematicAutorepeat = 0xFB,
    SetSpecificMakeRelease = 0xFC,
    SetSpecificMakeOnly = 0xFD,
    ResendLastByte = 0xFE,
    ResetAndStartSelfTest = 0xFF,
}

enum Response {
    KeyDetectionError = 0x00,
    SelfTestPassed = 0xAA,
    Echo = 0xEE,
    ACK = 0xFA,
    SelfTestFailed = 0xFC,
    SelfTestFailed2 = 0xFD,
    Resend = 0xFE,
    KeyDetectionError2 = 0xFF,
}

enum ScanCodeSetId {
    ScanCodeSet1,
    ScanCodeSet2,
    ScanCodeSet3,
}

const SCANCODE_SET_1: [char; 83] = [
    '\0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '-', '=', '\0',
    '\0', 'Q', 'W', 'E', 'R', 'T', 'Y', 'U', 'I', 'O', 'P', '[', ']', '\n',
    '\0', 'A', 'S', 'D', 'F', 'G', 'H', 'J', 'K', 'L', ';', '\'', '`', '\0', '\\',
    'Z', 'X', 'C', 'V', 'B', 'N', 'M', ',', '.', '/', '\0',
    '*', '0', ' ', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
    '\0', '\0', '7', '8', '9', '-', '4', '5', '6', '+', '1', '2', '3', '0', '.'
];

#[derive(Debug, Copy, Clone)]
pub struct PS2Keyboard {
    port: PS2Port,

    is_caps_lock: bool,
    is_num_lock: bool,
    is_scroll_lock: bool,

    is_lshift: bool,
    is_rshift: bool,
    is_lcontrol: bool,
    is_rcontrol: bool,
    is_lalt: bool,
    is_ralt: bool,

    is_reading_extended_keycode: bool,
}

impl PS2Keyboard {
    pub fn new(port: PS2Port) -> Self {
        let keyboard = Self {
            port,

            is_caps_lock: false,
            is_num_lock: false,
            is_scroll_lock: false,

            is_lshift: false,
            is_rshift: false,
            is_lcontrol: false,
            is_rcontrol: false,
            is_lalt: false,
            is_ralt: false,

            is_reading_extended_keycode: false,
        };

        keyboard
    }

    /// Directly reading a byte from the device port, this should only be called from an IRS to ensure that data is present
    pub fn interrupt_read_byte() -> u8 {
        DATA_PORT.lock().read().unwrap()
    }

    pub fn print_key_input(&mut self, scancode: u8) {
        match scancode {
            0x54..=0x56 | 0x59..=0x80 => (), // Not mapped, maybe want to ask to resend last byte?
            0x01 => (), // Escape pressed,
            0x1C => (), // Enter pressed TODO
            0x3B..=0x44 | 0x57 | 0x58 => (), // Fn keys pressed
            0x0E => vga_buffer::backspace(), // Backspace pressed
            0x0F => println!("  "), // Tab pressed
            0x1D => self.is_lcontrol = true,

            0x2A => self.is_lshift = true, // Left shift pressed
            0x36 => self.is_rshift = true, // Right shift pressed
            0x38 => self.is_lalt = true, // Left alt pressed
            0x3A => self.is_caps_lock = true, // Caps lock pressed
            0x45 => self.is_num_lock = true, // Num lock pressed
            0x46 => self.is_scroll_lock = true, // Scroll lock pressed

            0xAA => self.is_lshift = false, // Left shift released
            0xB6 => self.is_rshift = false, // Right shift released
            0xB8 => self.is_lalt = false, // Left all pressed
            0xC5 => self.is_num_lock = false, // Num lock pressed
            0xC6 => self.is_scroll_lock = false, // Scroll lock pressed

            0xE0 =>  {
                self.is_reading_extended_keycode = true;
                //self.print_key_input();
                self.is_reading_extended_keycode = false;
            }, // E

            _ => if scancode as usize <= SCANCODE_SET_1.len() {
                if self.is_caps() {
                    print!("{}", SCANCODE_SET_1[scancode as usize - 1]);
                }
                else {
                    print!("{}", SCANCODE_SET_1[scancode as usize - 1].to_lowercase());
                }
            }
        }
    }

    fn is_caps(&self) -> bool {
        !self.is_caps_lock != !(self.is_lshift | self.is_rshift)
    }
}

impl PS2Device for PS2Keyboard {
    fn device_type(&self) -> PS2DeviceType {
        MF2Keyboard
    }

    fn port(&self) -> PS2Port {
        self.port
    }
}
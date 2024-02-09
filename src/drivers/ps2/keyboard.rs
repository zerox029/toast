use bitflags::bitflags;
use crate::drivers::ps2::{PS2Device, PS2DeviceCommand, PS2DeviceType, PS2Port};
use crate::drivers::ps2::PS2DeviceType::MF2Keyboard;

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

#[derive(Debug, Copy, Clone)]
pub struct PS2Keyboard {
    port: PS2Port
}

impl PS2Keyboard {
    pub fn new(port: PS2Port) -> Self {
        Self {
            port
        }
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
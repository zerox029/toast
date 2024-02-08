use core::fmt;
use core::fmt::Formatter;
use lazy_static::lazy_static;
use spin::Mutex;
use crate::{println, print};
use crate::arch::x86_64::port_manager::Port;
use crate::arch::x86_64::port_manager::ReadWriteStatus::{ReadOnly, ReadWrite, WriteOnly};
use crate::drivers::ps2::PS2ControllerCommand::{DisableFirstPS2, DisableSecondPS2, EnableFirstPS2, EnableSecondPS2, ReadByteZero, TestFirstPS2, TestPS2Controller, TestSecondPS2, WriteToSecondPs2InputBuffer};
use crate::drivers::ps2::PS2Device::{AncientATKeyboard, FiveButtonMouse, HundredTwentyTwoKeyKeyboard, JapaneseAKeyboard, JapaneseGKeyboard, JapanesePKeyboard, MF2Keyboard, MouseWithScrollWheel, NCDN97Keyboard, NCDSunLayoutKeyboard, ShortKeyboard, StandardPS2Mouse};
use crate::drivers::ps2::PS2DeviceCommand::{ACK, DisableScanning, Identify, Reset, SelfTestSuccessful};
use crate::drivers::ps2::PS2Port::{FirstPS2Port, SecondPS2Port};
use crate::utils::bitutils::is_nth_bit_set;

const DATA_PORT_ADDRESS: u16 = 0x60;
const STATUS_REGISTER_ADDRESS: u16 = 0x64;
const COMMAND_REGISTER_ADDRESS: u16 = 0x64;

#[derive(Debug, Copy, Clone)]
enum PS2Port {
    FirstPS2Port,
    SecondPS2Port,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone)]
enum PS2ControllerCommand {
    ReadByteZero = 0x20,
    WriteToByteZero = 0x60,
    DisableSecondPS2 = 0xA7,
    EnableSecondPS2 = 0xA8,
    TestSecondPS2 = 0xA9,
    TestPS2Controller = 0xAA,
    TestFirstPS2 = 0xAB,
    DiagnosticDumb = 0xAC,
    DisableFirstPS2 = 0xAD,
    EnableFirstPS2 = 0xAE,
    ReadControllerInput = 0xC0,
    CopyBitsZeroToThree = 0xC1,
    CopyBitsFourToSeven = 0xC2,
    ReadControllerOutput = 0xD0,
    WriteToControllerOutput = 0xD1,
    WriteToFirstPS2OutputBuffer = 0xD2,
    WriteToSecondPS2OutputBuffer = 0xD3,
    WriteToSecondPs2InputBuffer = 0xD4,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone)]
enum PS2DeviceCommand {
    SelfTestSuccessful = 0xAA,
    Identify = 0xF2,
    EnableScanning = 0xF4,
    DisableScanning = 0xF5,
    ACK = 0xFA,
    Reset = 0xFF,
}

#[derive(Debug, Copy, Clone)]
enum PS2Device {
    AncientATKeyboard,
    StandardPS2Mouse,
    MouseWithScrollWheel,
    FiveButtonMouse,
    MF2Keyboard,
    ShortKeyboard,
    NCDN97Keyboard,
    HundredTwentyTwoKeyKeyboard,
    JapaneseGKeyboard,
    JapanesePKeyboard,
    JapaneseAKeyboard,
    NCDSunLayoutKeyboard,
}

impl fmt::Display for PS2Device {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

lazy_static! {
    pub static ref DATA_PORT: Mutex<Port<u8>> = Mutex::new(Port::new(DATA_PORT_ADDRESS, ReadWrite).into());
    pub static ref STATUS_REGISTER: Mutex<Port<u8>> = Mutex::new(Port::new(STATUS_REGISTER_ADDRESS, ReadOnly));
    pub static ref COMMAND_REGISTER: Mutex<Port<u8>> = Mutex::new(Port::new(COMMAND_REGISTER_ADDRESS, WriteOnly));
}

pub fn init_ps2_controller() {
    println!("Attempting to initialize PS/2 drivers...");

    if !check_ps2_controller_exists() {
        println!("Could not find PS/2 controller...");
        return;
    }

    disable_ps2_devices();
    flush_output_buffer();
    set_config_byte();
    controller_self_test();
    let is_dual_channel = dual_channel_check();
    let devices = interface_test(is_dual_channel);
    enable_devices(devices);
    reset_devices(devices);

    println!("Successfully initialized PS/2 drivers!");

    println!("Detected {}", detect_device(FirstPS2Port));
}

fn check_ps2_controller_exists() -> bool {
    true
}

fn disable_ps2_devices() {
    COMMAND_REGISTER.lock().write(DisableFirstPS2 as u8).unwrap();
    COMMAND_REGISTER.lock().write(DisableSecondPS2 as u8).unwrap();
}

fn flush_output_buffer() {
    DATA_PORT.lock().read().unwrap();
}

fn set_config_byte() {
    let mut config_byte = send_command_for_response(ReadByteZero);
    update_config_byte(config_byte & !0b00100011);
}

fn controller_self_test() {
    let config_byte = send_command_for_response(ReadByteZero);

    let response = send_command_for_response(TestPS2Controller);

    assert_eq!(response, 0x55);

    // Resetting the config byte for compatibility with some computers
    update_config_byte(config_byte);
}

fn dual_channel_check() -> bool {
    COMMAND_REGISTER.lock().write(EnableSecondPS2 as u8).unwrap();

    let mut config_byte = send_command_for_response(ReadByteZero);
    let dual_channel_bit = config_byte & (1 << 5) == 1;

    // Disable second PS/2 port if dual channel
    if !dual_channel_bit {
        COMMAND_REGISTER.lock().write(DisableSecondPS2 as u8).unwrap();
    }

    !dual_channel_bit
}

fn interface_test(is_dual_channel: bool) -> (bool, bool){
    let response = send_command_for_response(TestFirstPS2);

    if is_dual_channel {
        let second_response = send_command_for_response(TestSecondPS2);

        return (response == 0, second_response == 0)
    }

    (response == 0, false)
}

fn enable_devices(devices: (bool, bool)) {
    let mut byte_controller_bit_mask = 0;

    if devices.0 {
        COMMAND_REGISTER.lock().write(EnableFirstPS2 as u8).unwrap();
        byte_controller_bit_mask |= 0b00000001;
    }

    if devices.1 {
        COMMAND_REGISTER.lock().write(EnableSecondPS2 as u8).unwrap();
        byte_controller_bit_mask |= 0b00000010;
    }

    // Enable interrupts
    let mut config_byte = send_command_for_response(ReadByteZero);
    COMMAND_REGISTER.lock().write(config_byte | byte_controller_bit_mask).unwrap();

    wait_for_input_buffer();

    DATA_PORT.lock().write(config_byte).unwrap();
}

fn reset_devices(devices: (bool, bool)) {
    if devices.0 {
        write_to_device(Reset, FirstPS2Port);

        let second_response = read_from_device(FirstPS2Port);
        assert_eq!(second_response, SelfTestSuccessful as u8);
        DATA_PORT.lock().read().unwrap(); // I honestly cannot figure out why this is necessary
    }

    if devices.1 {
        write_to_device(Reset, SecondPS2Port);

        let second_response = read_from_device(SecondPS2Port);
        assert_eq!(second_response, SelfTestSuccessful as u8);
        DATA_PORT.lock().read().unwrap(); // Same as above
    }
}

fn detect_device(port: PS2Port) -> PS2Device {
    write_to_device(Reset, port);
    write_to_device(Identify, port);

    let first_byte = read_from_device(port);
    let second_byte = read_from_device(port);

    DATA_PORT.lock().read().unwrap(); // Same as above
    DATA_PORT.lock().read().unwrap(); // Same as above

    match first_byte {
        0x00 => StandardPS2Mouse,
        0x03 => MouseWithScrollWheel,
        0x04 => FiveButtonMouse,
        0xAB => match second_byte {
            0x41 | 0xC1 => MF2Keyboard,
            0x54 => ShortKeyboard,
            0x85 => NCDN97Keyboard,
            0x86 => HundredTwentyTwoKeyKeyboard,
            0x90 => JapaneseGKeyboard,
            0x91 => JapanesePKeyboard,
            0x92 => JapaneseAKeyboard,
            _ => panic!("Erroneous byte received")
        },
        0xAC => NCDSunLayoutKeyboard,
        _ => AncientATKeyboard,
    }
}


fn send_command_for_response(command: PS2ControllerCommand) -> u8 {
    COMMAND_REGISTER.lock().write(command as u8).unwrap();

    wait_for_output_buffer();

    DATA_PORT.lock().read().unwrap()
}

fn update_config_byte(config_byte: u8) {
    DATA_PORT.lock().write(config_byte).unwrap();

    wait_for_output_buffer();

    DATA_PORT.lock().read().unwrap();
}

fn write_to_device(command: PS2DeviceCommand, port: PS2Port) {
    match port {
        FirstPS2Port => {
            while is_nth_bit_set(STATUS_REGISTER.lock().read().unwrap(), 1) {}

            DATA_PORT.lock().write(command as u8).unwrap();

            let response = read_from_device(SecondPS2Port);
            assert_eq!(response, ACK as u8);
        },
        SecondPS2Port => {
            COMMAND_REGISTER.lock().write(WriteToSecondPs2InputBuffer as u8).unwrap();

            while is_nth_bit_set(STATUS_REGISTER.lock().read().unwrap(), 1) {}

            DATA_PORT.lock().write(command as u8).unwrap();

            let response = read_from_device(SecondPS2Port);
            assert_eq!(response, ACK as u8);
        }
    }
}

// TODO: Use interrupt method to avoid blocking the CPU
fn read_from_device(_port: PS2Port) -> u8 {
    while !is_nth_bit_set(STATUS_REGISTER.lock().read().unwrap(), 0) {}

    DATA_PORT.lock().read().unwrap()
}

// TODO: When multithreading, set a timeout here
fn wait_for_output_buffer() {
    while STATUS_REGISTER.lock().read().unwrap() & (1 << 0) == 0 {}
}

// TODO: When multithreading, set a timeout here
fn wait_for_input_buffer() {
    while STATUS_REGISTER.lock().read().unwrap() & (1 << 1) == 1 {}
}
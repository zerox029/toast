pub mod keyboard;

use alloc::boxed::Box;
use core::fmt;
use core::fmt::{Formatter, Debug};
use downcast_rs::{Downcast, impl_downcast};
use lazy_static::lazy_static;
use spin::Mutex;
use crate::{println, print};
use crate::arch::x86_64::port_manager::Port;
use crate::arch::x86_64::port_manager::ReadWriteStatus::*;
use crate::drivers::ps2::keyboard::PS2Keyboard;
use crate::drivers::ps2::PS2ControllerCommand::*;
use crate::drivers::ps2::PS2DeviceType::*;
use crate::drivers::ps2::PS2DeviceCommand::*;
use crate::drivers::ps2::PS2Port::*;
use crate::utils::bitutils::is_nth_bit_set;

const DATA_PORT_ADDRESS: u16 = 0x60;
const STATUS_REGISTER_ADDRESS: u16 = 0x64;
const COMMAND_REGISTER_ADDRESS: u16 = 0x64;

lazy_static! {
    pub static ref DATA_PORT: Mutex<Port<u8>> = Mutex::new(Port::new(DATA_PORT_ADDRESS, ReadWrite).into());
    pub static ref STATUS_REGISTER: Mutex<Port<u8>> = Mutex::new(Port::new(STATUS_REGISTER_ADDRESS, ReadOnly));
    pub static ref COMMAND_REGISTER: Mutex<Port<u8>> = Mutex::new(Port::new(COMMAND_REGISTER_ADDRESS, WriteOnly));
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PS2Port {
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
pub enum PS2DeviceCommand {
    SelfTestSuccessful = 0xAA,
    Identify = 0xF2,
    EnableScanning = 0xF4,
    DisableScanning = 0xF5,
    ACK = 0xFA,
    Reset = 0xFF,
}

#[derive(Debug, Copy, Clone)]
pub enum PS2DeviceType {
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
    Generic,
}

impl fmt::Display for PS2DeviceType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct GenericPS2Device {
    port: PS2Port,
}

pub trait PS2Device: Downcast {
    fn device_type(&self) -> PS2DeviceType;

    fn port(&self) -> PS2Port;

    fn read_byte(&self) -> u8 {
        while !is_nth_bit_set(STATUS_REGISTER.lock().read().unwrap() as usize, 0) {}

        DATA_PORT.lock().read().unwrap()
    }

    fn write_byte(&self, command: u8) {
        match self.port() {
            FirstPS2Port => {
                while is_nth_bit_set(STATUS_REGISTER.lock().read().unwrap() as usize, 1) {}

                DATA_PORT.lock().write(command).unwrap();

                let response = self.read_byte();
                assert_eq!(response, ACK as u8);
            },
            SecondPS2Port => {
                COMMAND_REGISTER.lock().write(WriteToSecondPs2InputBuffer as u8).unwrap();

                while is_nth_bit_set(STATUS_REGISTER.lock().read().unwrap() as usize, 1) {}

                DATA_PORT.lock().write(command).unwrap();

                let response = self.read_byte();
                assert_eq!(response, ACK as u8);
            }
        }
    }
}
impl_downcast!(PS2Device);

impl Debug for dyn PS2Device {
    fn fmt(&self, _f: &mut Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl PS2Device for GenericPS2Device {
    fn device_type(&self) -> PS2DeviceType {
        Generic
    }

    fn port(&self) -> PS2Port {
        self.port
    }
}

pub struct PS2ControllerDevices<T, S>(pub Option<T>, pub Option<S>);

pub fn init_ps2_controller() -> (Option<Box<dyn PS2Device>>, Option<Box<dyn PS2Device>>) {
    println!("Attempting to initialize PS/2 driver...");

    if !check_ps2_controller_exists() {
        println!("Could not find PS/2 controller...");
        return (None, None);
    }

    disable_ps2_devices();
    flush_output_buffer();
    set_config_byte();
    controller_self_test();
    let is_dual_channel = dual_channel_check();
    let devices = interface_test(is_dual_channel);
    enable_devices(&devices);
    reset_devices(&devices);

    println!("Successfully initialized PS/2 driver!");

    let first_port_device = detect_device(&devices.0.unwrap());

    println!("Detected {}", first_port_device.as_ref().unwrap().device_type());

    (first_port_device, None)
}


fn check_ps2_controller_exists() -> bool {
    // TODO: Since we use ACPIv1, the required data is not present in the FADT table, I'm not quite sure what to do of this situation
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
    let config_byte = send_command_for_response(ReadByteZero);
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

    let config_byte = send_command_for_response(ReadByteZero);
    let dual_channel_bit = config_byte & (1 << 5) == 1;

    // Disable second PS/2 port if dual channel
    if !dual_channel_bit {
        COMMAND_REGISTER.lock().write(DisableSecondPS2 as u8).unwrap();
    }

    !dual_channel_bit
}

fn interface_test(is_dual_channel: bool) -> PS2ControllerDevices<GenericPS2Device, GenericPS2Device> {
    let first_response = send_command_for_response(TestFirstPS2);
    let first_device = if first_response == 0 { Some(GenericPS2Device { port: FirstPS2Port }) } else { None };

    if is_dual_channel {
        let second_response = send_command_for_response(TestSecondPS2);
        let second_device = if second_response == 0 { Some(GenericPS2Device { port: SecondPS2Port }) } else { None };

        return PS2ControllerDevices(first_device, second_device)
    }

    PS2ControllerDevices(first_device, None)
}

fn enable_devices(devices: &PS2ControllerDevices<GenericPS2Device, GenericPS2Device>) {
    let mut byte_controller_bit_mask = 0b00000000;

    if devices.0.is_some() {
        COMMAND_REGISTER.lock().write(EnableFirstPS2 as u8).unwrap();
        byte_controller_bit_mask |= 0b00000001;
    }

    if devices.1.is_some() {
        COMMAND_REGISTER.lock().write(EnableSecondPS2 as u8).unwrap();
        byte_controller_bit_mask |= 0b00000010;
    }

    // Enable interrupts
    let config_byte = send_command_for_response(ReadByteZero);
    COMMAND_REGISTER.lock().write(config_byte | byte_controller_bit_mask).unwrap();

    wait_for_input_buffer();

    DATA_PORT.lock().write(config_byte).unwrap();
}

fn reset_devices(devices: &PS2ControllerDevices<GenericPS2Device, GenericPS2Device>) {
    if devices.0.is_some() {
        let device = devices.0.as_ref().unwrap();
        device.write_byte(Reset as u8);

        let second_response = device.read_byte();
        assert_eq!(second_response, SelfTestSuccessful as u8);
        DATA_PORT.lock().read().unwrap(); // I honestly cannot figure out why this is necessary, but it doesn't work without
    }

    if devices.1.is_some() {
        let device = devices.1.as_ref().unwrap();
        device.write_byte(Reset as u8);

        let second_response = device.read_byte();
        assert_eq!(second_response, SelfTestSuccessful as u8);
        DATA_PORT.lock().read().unwrap(); // Same as above
    }
}

fn detect_device(generic_device: &GenericPS2Device) -> Option<Box<dyn PS2Device>> {
    generic_device.write_byte(Reset as u8);
    generic_device.write_byte(Identify as u8);

    let first_byte = generic_device.read_byte();
    let second_byte = generic_device.read_byte();

    DATA_PORT.lock().read().unwrap(); // Same as above
    DATA_PORT.lock().read().unwrap(); // Same as above

    match first_byte {
        0xAB => match second_byte {
            0x41 | 0xC1 => Some(Box::new(PS2Keyboard::new(generic_device.port()))),
            _ => None
        },
        _ => None,
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

// TODO: When multithreading, set a timeout here
fn wait_for_output_buffer() {
    while STATUS_REGISTER.lock().read().unwrap() & (1 << 0) == 0 {}
}

// TODO: When multithreading, set a timeout here
fn wait_for_input_buffer() {
    while STATUS_REGISTER.lock().read().unwrap() & (1 << 1) == 1 {}
}
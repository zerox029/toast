use bitflags::bitflags;
use lazy_static::lazy_static;
use spin::Mutex;
use crate::{println, print};
use crate::arch::x86_64::port_manager::Port;
use crate::arch::x86_64::port_manager::ReadWriteStatus::{ReadOnly, ReadWrite, WriteOnly};
use crate::utils::bitutils::is_nth_bit_set;

const DATA_PORT_ADDRESS: u16 = 0x60;
const STATUS_REGISTER_ADDRESS: u16 = 0x64;
const COMMAND_REGISTER_ADDRESS: u16 = 0x64;

bitflags! {
    pub struct ConfigurationByteFlags: u8 {
        const FIRST_PS2_PORT_INTERRUPT = 1 << 0;
        const SECOND_PS2_PORT_INTERRUPT = 1 << 2;
        const SYSTEM = 1 << 3;
        const FIRST_PS2_PORT_CLOCK = 1 << 4;
        const SECOND_PS2_PORT_CLOCK = 1 << 5;
        const FIRST_PS2_PORT_TRANSLATION = 1 << 6;
    }

    pub struct StatusRegisterFlags: u8 {
        const OUTPUT_BUFFER_STATUS = 1 << 0;
        const INPUT_BUFFER_STATUS = 1 << 1;
        const SYSTEM = 1 << 2;
        const COMMAND_DATA = 1 << 3;
        const UNKNOWN = 1 << 4;
        const UNKNOWN_2 = 1 << 5;
        const TIME_OUT_ERROR = 1 << 6;
        const PARITY_ERROR = 1 << 7;
    }
}

lazy_static! {
    pub static ref DATA_PORT: Mutex<Port<u8>> = Mutex::new(Port::new(DATA_PORT_ADDRESS, ReadWrite).into());
    pub static ref STATUS_REGISTER: Mutex<Port<u8>> = Mutex::new(Port::new(STATUS_REGISTER_ADDRESS, ReadOnly));
    pub static ref COMMAND_REGISTER: Mutex<Port<u8>> = Mutex::new(Port::new(COMMAND_REGISTER_ADDRESS, WriteOnly));
}

pub fn init_ps2_controller() {
    println!("Attempting to initiate PS/2 drivers...");

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
}

fn check_ps2_controller_exists() -> bool {
    true
}

fn disable_ps2_devices() {
    COMMAND_REGISTER.lock().write(0xAD).unwrap();
    COMMAND_REGISTER.lock().write(0xA7).unwrap();
}

fn flush_output_buffer() {
    DATA_PORT.lock().read().unwrap();
}

fn set_config_byte() {
    COMMAND_REGISTER.lock().write(0x20).unwrap();

    wait_for_output_buffer();

    let mut config_byte = DATA_PORT.lock().read().unwrap();
    COMMAND_REGISTER.lock().write(config_byte & !0b00100011).unwrap();

    wait_for_input_buffer();

    DATA_PORT.lock().write(config_byte).unwrap();

    wait_for_output_buffer();

    DATA_PORT.lock().read().unwrap();
}

fn controller_self_test() {
    COMMAND_REGISTER.lock().write(0xAA).unwrap();

    wait_for_output_buffer();

    let response = DATA_PORT.lock().read().unwrap();

    assert_eq!(response, 0x55);

    // TODO: Restore controller configuration byte for compatibility
}

fn dual_channel_check() -> bool {
    COMMAND_REGISTER.lock().write(0xA8).unwrap();

    COMMAND_REGISTER.lock().write(0x20).unwrap();

    wait_for_output_buffer();

    let mut config_byte = DATA_PORT.lock().read().unwrap();
    let dual_channel_bit = config_byte & (1 << 5) == 1;

    // Disable second PS/2 port if dual channel
    if !dual_channel_bit {
        COMMAND_REGISTER.lock().write(0xA7).unwrap();
    }

    !dual_channel_bit
}

fn interface_test(is_dual_channel: bool) -> (bool, bool){
    COMMAND_REGISTER.lock().write(0xAB).unwrap();

    wait_for_output_buffer();

    let response = DATA_PORT.lock().read().unwrap();

    if is_dual_channel {
        COMMAND_REGISTER.lock().write(0xA9).unwrap();

        wait_for_output_buffer();

        let second_response = DATA_PORT.lock().read().unwrap();

        return (response == 0, second_response == 0)
    }

    (response == 0, false)
}

fn enable_devices(devices: (bool, bool)) {
    let mut byte_controller_bit_mask = 0;

    if devices.0 {
        COMMAND_REGISTER.lock().write(0xAE).unwrap();
        byte_controller_bit_mask |= 0b00000001;
    }

    if devices.1 {
        COMMAND_REGISTER.lock().write(0xA8).unwrap();
        byte_controller_bit_mask |= 0b00000010;
    }

    // Enable interrupts
    COMMAND_REGISTER.lock().write(0x20).unwrap();

    wait_for_output_buffer();

    let mut config_byte = DATA_PORT.lock().read().unwrap();
    COMMAND_REGISTER.lock().write(config_byte | byte_controller_bit_mask).unwrap();

    wait_for_input_buffer();

    DATA_PORT.lock().write(config_byte).unwrap();
}

fn reset_devices(devices: (bool, bool)) {
    if devices.0 {
        while is_nth_bit_set(STATUS_REGISTER.lock().read().unwrap(), 1) {}

        DATA_PORT.lock().write(0xFF).unwrap();

        while !is_nth_bit_set(STATUS_REGISTER.lock().read().unwrap(), 0) {}

        let response = DATA_PORT.lock().read().unwrap();

        assert_eq!(response, 0xFA);

        let second_response = DATA_PORT.lock().read().unwrap();

        assert_eq!(second_response, 0xAA);
    }

    if devices.1 {
        COMMAND_REGISTER.lock().write(0xD4).unwrap();

        while is_nth_bit_set(STATUS_REGISTER.lock().read().unwrap(), 1) {}

        DATA_PORT.lock().write(0xFF).unwrap();

        while !is_nth_bit_set(STATUS_REGISTER.lock().read().unwrap(), 0) {}

        let response = DATA_PORT.lock().read().unwrap();

        assert_eq!(response, 0xFA);

        let second_response = DATA_PORT.lock().read().unwrap();

        assert_eq!(second_response, 0xAA);
    }
}

fn wait_for_output_buffer() {
    while STATUS_REGISTER.lock().read().unwrap() & (1 << 0) == 0 {}
}

fn wait_for_input_buffer() {
    while STATUS_REGISTER.lock().read().unwrap() & (1 << 1) == 1 {}
}
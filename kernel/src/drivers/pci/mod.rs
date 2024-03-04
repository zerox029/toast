use alloc::vec::Vec;
use spin::Mutex;
use crate::arch::x86_64::port_manager::Port;
use crate::arch::x86_64::port_manager::ReadWriteStatus::ReadWrite;
use crate::{vga_print, info};
use crate::utils::bitutils::is_nth_bit_set;

pub mod ahci;

const CONFIG_ADDRESS: u16 = 0xCF8;
const CONFIG_DATA: u16 = 0xCFC;

static CONFIG_ADDRESS_PORT: Mutex<Port<u32>> = Mutex::new(Port::new(CONFIG_ADDRESS, ReadWrite));
static CONFIG_DATA_PORT: Mutex<Port<u32>> = Mutex::new(Port::new(CONFIG_DATA, ReadWrite));

#[derive(Debug, Copy, Clone)]
pub struct PCIDevice {
    pub bus: u8,
    pub device: u8,
}

// TODO: Support multifunction devices
impl PCIDevice {
    pub fn device_id(&self, function: u8) -> u16 {
        let header_field = config_read_word(self.bus, self.device, function, 0);
        ((header_field & 0xFFFF0000) >> 16) as u16
    }

    pub fn vendor_id(&self, function: u8) -> u16 {
        let header_field = config_read_word(self.bus, self.device, function, 0);
        (header_field & 0x0000FFFF) as u16
    }

    pub fn command(&self, function: u8) -> u16 {
        let header_field = config_read_word(self.bus, self.device, function, 0x4);
        (header_field & 0x0000FFFF) as u16
    }

    pub fn set_command(&self, function: u8, value: u16) {
        let value = value as u32 | ((self.status(function) as u32) << 16);
        config_write_word(self.bus, self.device, function, 0x4, value);
    }

    pub fn status(&self, function: u8) -> u16 {
        let header_field = config_read_word(self.bus, self.device, function, 0x4);
        ((header_field & 0xFFFF0000) >> 16) as u16
    }

    pub fn class_code(&self, function: u8) -> u16 {
        let header_field = config_read_word(self.bus, self.device, function, 0x8);
        ((header_field & 0xFF000000) >> 24) as u16
    }

    pub fn subclass(&self, function: u8) -> u16 {
        let header_field = config_read_word(self.bus, self.device, function, 0x8);
        ((header_field & 0x00FF0000) >> 16) as u16
    }

    pub fn header_type(&self, function: u8) -> u16 {
        let header_field = config_read_word(self.bus, self.device, function, 0xC);
        ((header_field & 0x00FF0000) >> 16) as u16
    }

    pub fn bar5(&self, function: u8) -> u32 {
        config_read_word(self.bus, self.device, function, 0x24)
    }

    pub fn interrupt_line(&self, function: u8) -> u8 {
        let header_field = config_read_word(self.bus, self.device, function, 0x3C);
        (header_field & 0x000000FF) as u8
    }

    pub fn check_device(&self) -> Vec<PCIDevice> {
        let mut devices = Vec::new();

        if self.vendor_id(0) == 0xFFFF {
            return devices;
        }

        devices.extend(self.check_function(0));

        if is_nth_bit_set(self.header_type(0) as usize, 7) {
            for function in 1..=7 {
                if self.vendor_id(function) != 0xFFFF {
                    devices.extend(self.check_function(function));
                }
            }
        }

        devices
    }

    fn check_function(&self, function: u8) -> Vec<PCIDevice> {
        if self.class_code(function) == 0x6 && self.subclass(function) == 0x4 {
            let secondary_bus = (config_read_word(self.bus, self.device, 0, 0x18) & 0x0000FF00 >> 8) as u8;
            return check_bus(secondary_bus);
        }

        Vec::new()
    }
}

impl PCIDevice {
    pub fn new(bus: u8, device: u8) -> Self {
        Self {
            bus,
            device,
        }
    }
}

pub fn check_all_buses() {
    let mut devices = Vec::new();

    let header_device = PCIDevice::new(0, 0);
    if is_nth_bit_set(header_device.header_type(0) as usize, 7) {
        // Single PCI host controller
        devices.extend(check_bus(0));
    }
    else {
        // Multiple PCI host controllers
        for function in 0..=7 {
            if header_device.vendor_id(function) != 0xFFFF {
                break;
            }

            devices.extend(check_bus(function));
        }
    }

    devices.iter().for_each(|device| info!("Device ID {:X}   Vendor ID: {:X}", device.device_id(0), device.vendor_id(0)));
}

fn check_bus(bus: u8) -> Vec<PCIDevice> {
    let mut pci_devices = Vec::new();

    for device_number in 0..=31 {
        let device = PCIDevice::new(bus, device_number);
        pci_devices.extend(device.check_device())
    }

    pci_devices
}

// Todo: Get the recursive method to work instead
pub fn find_all_pci_devices() -> Vec<PCIDevice> {
    let mut pci_devices = Vec::new();

    for bus in 0..=255 {
        for device in 0..=31 {
            if let Some(found_device) = get_device_if_exists(bus, device) {
                pci_devices.push(found_device);
            }
        }
    }

    pci_devices
}

fn get_device_if_exists(bus: u8, device_number: u8) -> Option<PCIDevice> {
    let device = PCIDevice::new(bus, device_number);
    if device.vendor_id(0) == 0xFFFF { return None; }

    Some(device)
}

fn config_read_word(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    let address = build_config_address(bus, slot, func, offset);

    CONFIG_ADDRESS_PORT.lock().write(address).unwrap();

    CONFIG_DATA_PORT.lock().read().unwrap()
}

fn config_write_word(bus: u8, slot: u8, func: u8, offset: u8, value: u32) {
    let address = build_config_address(bus, slot, func, offset);

    CONFIG_ADDRESS_PORT.lock().write(address).unwrap();
    CONFIG_DATA_PORT.lock().write(value).unwrap();
}

fn build_config_address(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    ((bus as u32) << 16) | ((slot as u32) << 11) | ((func as u32) << 8) | ((offset as u32) & 0xFC) | 0x80000000u32
}

use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use limine::framebuffer::Framebuffer;
use rlibc::memcpy;
use spin::Mutex;
use crate::fs::{Vfs, VfsNode, VfsNodeRef, VfsNodeWeakRef};
use crate::memory::{PhysicalAddress, VirtualAddress};

lazy_static! {
    pub static ref FB_DEVICES: Mutex<Vec<FrameBufferDevice>> = Mutex::new(Vec::new());
}

#[derive(Clone)]
pub struct FrameBufferDevice {
    name: String,
    parent: Option<VfsNodeWeakRef>,
    children: Vec<VfsNodeRef>,
    pub screen_info: FrameBufferScreenInfo,
}

#[derive(Clone)]
pub struct FrameBufferScreenInfo {
    pub address: VirtualAddress,
    pub width: u64,
    pub height: u64,
    pub pitch: u64,
    pub bpp: u16,
}

impl FrameBufferDevice {
    /// Initialize a framebuffer device and add it to the list
    pub fn init(framebuffer: &Framebuffer, name: String) {
        let screen_info = FrameBufferScreenInfo {
            address: framebuffer.addr() as PhysicalAddress,
            width: framebuffer.width(),
            height: framebuffer.height(),
            pitch: framebuffer.pitch(),
            bpp: framebuffer.bpp(),
        };

        let device = Self {
            name,
            parent: None,
            children: Vec::new(),
            screen_info
        };

        FB_DEVICES.lock().push(device);
    }

    /// Registers all framebuffer devices previously initialized by adding them to the vfs
    pub fn register_devices() {
        let parent = Vfs::find_from_absolute_path("/dev").expect("fs: could not find /dev");

        let devices = FB_DEVICES.lock();
        devices.iter().for_each(|device| {
            serial_println!("{}", device.name);
            // Not sure cloning is the best idea here
            let fbdev = Arc::new(Mutex::new(Box::new(device.clone()) as Box<dyn VfsNode + Send>));
            Vfs::insert_child_node(parent.clone(), fbdev);
        });
    }
}

impl VfsNode for FrameBufferDevice {
    fn name(&self) -> &String {
        &self.name
    }

    fn parent(&self) -> &Option<VfsNodeWeakRef> {
        &self.parent
    }

    fn children(&mut self) -> &mut Vec<VfsNodeRef> {
        &mut self.children
    }

    fn open(&self) {
        todo!()
    }

    fn close(&self) {
        todo!()
    }

    fn read(&self, _buffer: *mut u8, _byte_count: usize, _offset: usize) {
        todo!()
    }

    fn write(&self, buffer: *const u8, byte_count: usize, offset: usize) {
        unsafe { memcpy((self.screen_info.address + offset) as *mut u8, buffer, byte_count) };
    }
}
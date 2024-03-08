use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ffi::c_void;
use limine::framebuffer::Framebuffer;
use rlibc::memcpy;
use spin::Mutex;
use crate::fs::{Vfs, VfsNode, VfsNodeRef, VfsNodeWeakRef};
use crate::memory::{PhysicalAddress, VirtualAddress};

pub struct FrameBufferDevice {
    name: String,
    parent: Option<VfsNodeWeakRef>,
    children: Vec<VfsNodeRef>,
    screen_info: FrameBufferScreenInfo,
}

pub struct FrameBufferScreenInfo {
    address: VirtualAddress,
    width: u64,
    height: u64,
    pitch: u64,
    bpp: u16,
}

impl FrameBufferDevice {
    pub fn register(framebuffer: &Framebuffer, name: String) {
        let screen_info = FrameBufferScreenInfo {
            address: framebuffer.addr() as PhysicalAddress,
            width: framebuffer.width(),
            height: framebuffer.height(),
            pitch: framebuffer.pitch(),
            bpp: framebuffer.bpp(),
        };

        let parent = Vfs::find_from_absolute_path("/dev").expect("fs: could not find /dev");
        let fbdev = Arc::new(Mutex::new(Box::new(Self {
            name,
            parent: Some(Arc::downgrade(&parent)),
            children: Vec::new(),
            screen_info
        }) as Box<dyn VfsNode + Send>));

        Vfs::insert_child_node(parent, fbdev);
    }
}

impl VfsNode for FrameBufferDevice {
    fn name(&self) -> &String {
        todo!()
    }

    fn parent(&self) -> &Option<VfsNodeWeakRef> {
        todo!()
    }

    fn children(&mut self) -> &mut Vec<VfsNodeRef> {
        todo!()
    }

    fn open(&self) {
        todo!()
    }

    fn close(&self) {
        todo!()
    }

    fn read(&self, buffer: *mut u8, byte_count: usize, offset: usize) {
        todo!()
    }

    fn write(&self, buffer: *const u8, byte_count: usize, offset: usize) {
        unsafe { memcpy((self.screen_info.address + offset) as *mut u8, buffer, byte_count) };
    }
}
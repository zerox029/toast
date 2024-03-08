use alloc::string::String;
use alloc::vec::Vec;
use crate::fs::{VfsNode, VfsNodeRef, VfsNodeWeakRef};

pub struct RamfsNode {
    pub(super) name: String,
    pub(super) parent: Option<VfsNodeWeakRef>,
    pub(super) children: Vec<VfsNodeRef>,
}

impl VfsNode for RamfsNode {
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
        panic!("fs: cannot invoke method 'open' a ramfs node");
    }

    fn close(&self) {
        panic!("fs: cannot invoke method 'close' on a ramfs node");
    }

    fn read(&self, buffer: *mut u8, byte_count: usize, offset: usize) {
        todo!()
    }

    fn write(&self, buffer: *const u8, byte_count: usize, offset: usize) {
        todo!()
    }
}
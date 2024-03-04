use alloc::string::String;
use alloc::vec::Vec;

pub struct Vfs {
    root_node: VfsNode,
    mount_points: Vec<VfsNode>
}

pub struct VfsNode {
    name: String,
    uuid: u128,
}

pub fn init() {
    let root_node = VfsNode {
        name: String::from("/"),
    };
}
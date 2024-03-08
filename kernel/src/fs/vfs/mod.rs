use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use conquer_once::spin::OnceCell;
use spin::Mutex;

type VfsNodeRef = Arc<Mutex<VfsNode>>;
type VfsNodeWeakRef = Weak<Mutex<VfsNode>>;

static ROOT_DIRECTORY: OnceCell<VfsNodeRef> = OnceCell::uninit();

pub struct Vfs {
    mount_points: Vec<VfsNode>
}

impl Vfs {
    pub fn init() {
        ROOT_DIRECTORY.init_once(|| {
            let mut root_node = Arc::new(Mutex::new(VfsNode {
                name: String::from("/"),
                parent: None,
                children: Vec::new(),
            }));

            let current_directory: VfsNodeRef = Arc::new(Mutex::new(VfsNode {
                name: String::from("."),
                parent: Some(Arc::downgrade(&root_node)),
                children: Vec::new(),
            }));

            let previous_directory: VfsNodeRef = Arc::new(Mutex::new(VfsNode {
                name: String::from(".."),
                parent: Some(Arc::downgrade(&root_node)),
                children: Vec::new(),
            }));

            {
                let mut root_node = root_node.lock();

                root_node.children.push(current_directory);
                root_node.children.push(previous_directory);
            }

            root_node
        });
    }

    pub fn root_directory() -> &'static VfsNodeRef {
        ROOT_DIRECTORY.try_get().expect("fs: virtual file system not initialized")
    }

    pub fn find_child(node: &VfsNodeRef, name: &str) -> Option<VfsNodeRef> {
        unimplemented!();
    }

    pub fn find_from_path(&self, path: &str) -> Option<VfsNodeRef> {
        unimplemented!();
    }

    pub fn parent(&self) -> &Option<VfsNodeWeakRef> {
        unimplemented!();
    }

    pub fn get_path(&self, ) -> String {
        unimplemented!();
    }
}

pub struct VfsNode {
    pub name: String,
    pub parent: Option<VfsNodeWeakRef>,
    pub children: Vec<VfsNodeRef>,
}
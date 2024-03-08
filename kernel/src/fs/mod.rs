use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::ffi::c_void;
use core::ops::ControlFlow;
use conquer_once::spin::OnceCell;
use spin::Mutex;
use crate::fs::ramfs::RamfsNode;

pub mod ext2;
pub mod ramfs;

const MAX_FILENAME_LENGTH: usize = 256;
const MAX_PATH_LENGTH: usize = 4096;

pub(crate) type VfsNodeRef = Arc<Mutex<Box<dyn VfsNode + Send>>>;
pub(crate) type VfsNodeWeakRef = Weak<Mutex<Box<dyn VfsNode + Send>>>;

static ROOT_DIRECTORY: OnceCell<VfsNodeRef> = OnceCell::uninit();

pub trait VfsNode {
    fn name(&self) -> &String;
    fn parent(&self) -> &Option<VfsNodeWeakRef>;
    fn children(&mut self) -> &mut Vec<VfsNodeRef>;

    fn open(&self, );
    fn close(&self, );

    fn read(&self, buffer: *mut u8, byte_count: usize, offset: usize);
    fn write(&self, buffer: *const u8, byte_count: usize, offset: usize);
}

pub struct Vfs {
    mount_points: Vec<RamfsNode>
}

impl Vfs {
    pub fn init() {
        ROOT_DIRECTORY.init_once(|| {
            let root_node = Arc::new(Mutex::new(Box::new(RamfsNode {
                name: String::from("/"),
                parent: None,
                children: Vec::new(),
            }) as Box<dyn VfsNode + Send>));

            let current_directory = Arc::new(Mutex::new(Box::new(RamfsNode {
                name: String::from("."),
                parent: Some(Arc::downgrade(&root_node)),
                children: Vec::new(),
            }) as Box<dyn VfsNode + Send>));

            let previous_directory = Arc::new(Mutex::new(Box::new(RamfsNode {
                name: String::from(".."),
                parent: Some(Arc::downgrade(&root_node)),
                children: Vec::new(),
            }) as Box<dyn VfsNode + Send>));

            let dev_directory =  Arc::new(Mutex::new(Box::new(RamfsNode {
                name: String::from("dev"),
                parent: Some(Arc::downgrade(&root_node)),
                children: Vec::new(),
            }) as Box<dyn VfsNode + Send>));

            {
                let mut root_node = root_node.lock();

                root_node.children().push(current_directory);
                root_node.children().push(previous_directory);
                root_node.children().push(dev_directory);
            }

            root_node
        });
    }

    pub fn root_directory() -> &'static VfsNodeRef {
        ROOT_DIRECTORY.try_get().expect("fs: virtual file system not initialized")
    }

    /// Creates a new ramfs node with the specified characteristics and adds it to the designated
    /// parent
    pub fn create_child_node(parent: VfsNodeRef, name: &str) {
        let child = Arc::new(Mutex::new(Box::new(RamfsNode {
            name: String::from(name),
            parent: Some(Arc::downgrade(&parent)),
            children: Vec::new(),
        }) as Box<dyn VfsNode + Send> ));

        Self::insert_child_node(parent, child);
    }

    /// Inserts the given node as a child of the designated parent
    pub fn insert_child_node(parent: VfsNodeRef, child: VfsNodeRef) {
        parent.lock().children().push(child);
    }

    /// Finds a node with the specified name in the children of the given node.
    pub fn find_child(node: VfsNodeRef, name: &str) -> Option<VfsNodeRef> {
        node.lock().children().iter().find(|child| child.lock().name() == name).map(|child| child.clone())
    }

    /// Finds a node at the specified path starting at the given node.
    /// Given the path "Desktop/someFolder" and the node "/home/user", it will return the node at
    /// "/home/user/Desktop/someFolder"
    pub fn find_descendent(node: VfsNodeRef, path: &str) -> Option<VfsNodeRef> {
        let mut path_iter = path[1..].split('/');

        let current_node = node;
        let node = path_iter.try_fold(current_node, |current_node, current_name| {
            if let Some(found_node) = Self::find_child(current_node, current_name) {
                ControlFlow::Continue(found_node)
            }
            else {
                ControlFlow::Break(())
            }
        });

        match node {
            ControlFlow::Continue(node) => Some(node),
            ControlFlow::Break(()) => None,
        }
    }

    pub fn find_from_absolute_path(path: &str) -> Option<VfsNodeRef> {
        Self::find_descendent(Self::root_directory().clone(), path)
    }

    /// Returns the parent of a given node
    pub fn parent(node: VfsNodeRef) -> Option<VfsNodeWeakRef> {
        node.lock().parent().clone()
    }

    /// Returns the absolute path of the given node
    pub fn get_absolute_path(node: VfsNodeRef) -> String {
        if node.lock().name() == "/" {
            return String::from("/");
        }

        let mut current_node = node.clone();
        let mut directory_entries: Vec<String> = Vec::new();
        directory_entries.push({
            let current_node = current_node.lock();

            format!("{}", current_node.name())
        });

        while let Some(parent) = current_node.clone().lock().parent().clone() {
            current_node = parent.upgrade().expect("lol get fucked");

            directory_entries.insert(0, format!("{}", current_node.lock().name()));
        }

        directory_entries.iter().skip(1).map(|entry| format!("/{}", entry) ).collect()
    }
}
use crate::{set_bit_to, test_bit};

/// A bitmap encoded binary tree. Each node is represented by a single bit stored contiguously
/// in breadth-first order
pub struct BitmapBinaryTree {
    /// A pointer to the first node in the tree
    nodes: *mut u8,
    /// The number of leaf nodes contained in the tree
    size: usize,
}
impl BitmapBinaryTree {
    pub fn new(start: *mut u8, size: usize) -> Self {
        Self {
            nodes: start,
            size,
        }
    }

    pub fn get_node(&self, index: usize) -> bool {
        let byte_index = index / 8;
        let bit_index = index % 8;

        let containing_byte = unsafe { self.nodes.add(byte_index) };

        test_bit!(containing_byte, bit_index)
    }

    pub fn set_node(&self, index: usize, value: bool) {
        let byte_index = index / 8;
        let bit_index = index % 8;

        let containing_byte = unsafe { self.nodes.add(byte_index) };

        set_bit_to!(containing_byte, bit_index, value);
    }

    pub fn get_children_indices(&self, index: usize) -> (usize, usize) {
        ((2 * index) + 1, (2 * index) + 2)
    }

    /// Returns the height of the tree
    pub fn get_height(&self) -> usize {
        (self.size as f64).log2().ceil() as usize
    }

    pub fn get_full_size(&self) -> usize {
        self.size * 2 - 1
    }
}
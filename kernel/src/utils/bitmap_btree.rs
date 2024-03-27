use crate::{set_bit_to, test_bit};

/// A statically sized bitmap encoded binary tree. Each node is represented by a single bit
/// stored contiguously in breadth-first order
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

    /// Gets the value of the node at the given index
    pub fn get_node(&self, index: usize) -> Result<bool, &'static str> {
        self.bound_check(index)?;

        let byte_index = index / 8;
        let bit_index = index % 8;

        let containing_byte = unsafe { self.nodes.add(byte_index) };

        Ok(test_bit!(unsafe { *containing_byte }, bit_index as u8))
    }

    /// Set the value of the node at the given index
    pub fn set_node(&self, index: usize, value: bool) -> Result<(), &'static str>{
        self.bound_check(index)?;

        let byte_index = index / 8;
        let bit_index = index % 8;

        let containing_byte = unsafe { self.nodes.add(byte_index) };

        Ok(set_bit_to!(unsafe { *containing_byte }, bit_index, value))
    }

    /// Get the indices of the two children nodes of the node at the given index
    pub fn get_children_indices(&self, index: usize) -> Result<Option<(usize, usize)>, &'static str> {
        self.bound_check(index)?;

        let left_child_index = (2 * index) + 1;
        let right_child_index = (2 * index) + 1;

        if left_child_index >= self.len() || right_child_index >= self.len() {
            return Ok(None);
        }

        Ok(Some((left_child_index, right_child_index)))
    }

    /// Returns the height of the tree
    pub fn get_height(&self) -> usize {
        todo!()
    }

    /// Returns the number of elements in the tree
    pub fn len(&self) -> usize {
        self.size * 2 - 1
    }


    fn bound_check(&self, index: usize) -> Result<(), &'static str> {
        if index >= self.len() {
            return Err("requested index outside tree boundary");
        }

        Ok(())
    }
}
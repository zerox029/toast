pub fn is_nth_bit_set(number: usize, bit_index: usize) -> bool {
    number & (1 << bit_index) != 0
}
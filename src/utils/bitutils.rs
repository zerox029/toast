pub fn is_nth_bit_set(number: u8, bit_index: u8) -> bool {
    number & (1 << bit_index) != 0
}
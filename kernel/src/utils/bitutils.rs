pub fn is_nth_bit_set(number: usize, bit_index: usize) -> bool {
    number & (1 << bit_index) != 0
}

#[macro_export]
macro_rules! set_bit {
    ($num:expr, $n:expr) => {{
        let mask = 1 << $n;
        $num | mask
    }};
}
use core::arch::asm;

pub fn rsp() -> usize {
    let rsp: usize;
    unsafe {
        asm! {
        "mov {}, rsp",
        out(reg) rsp,
        }
    }

    rsp
}

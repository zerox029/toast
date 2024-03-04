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

pub fn cr3() -> usize {
    let cr3: usize;
    unsafe {
        asm! {
            "mov {}, cr3",
            out(reg) cr3,
        }
    }

    cr3
}
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

pub fn cr0() -> usize {
    let cr0: usize;
    unsafe {
        asm! {
        "mov {}, cr0",
        out(reg) cr0,
        }
    }

    cr0
}

pub fn cr2() -> usize {
    let cr2: usize;
    unsafe {
        asm! {
        "mov {}, cr2",
        out(reg) cr2,
        }
    }

    cr2
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

pub fn cr4() -> usize {
    let cr4: usize;
    unsafe {
        asm! {
        "mov {}, cr4",
        out(reg) cr4,
        }
    }

    cr4
}

pub fn efer() -> usize {
    let cr4: usize;
    unsafe {
        asm! {
        "mov {}, cr4",
        out(reg) cr4,
        }
    }

    cr4
}
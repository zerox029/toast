use alloc::boxed::Box;
use core::arch::{asm};
use core::mem::size_of;
use bitfield::bitfield;
use crate::{println, print};

bitfield! {
    #[derive(Default)]
    struct SegmentDescriptor(u64);
    limit_low, set_limit_low: 15, 0;
    base_low, set_base_low: 31, 16;
    base_mid, set_base_mid: 39, 32;
    access_byte, set_access_byte: 47, 40;
    limit_high, set_limit_high: 51, 48;
    flags, set_flags: 55, 52;
    base_high, set_base_high: 63, 56;
}

bitfield! {
    #[derive(Default)]
    struct TssDescriptor(u128);
    limit_low, set_limit_low: 15, 0;
    base_low, set_base_low: 31, 16;
    base_mid, set_base_mid: 39, 32;
    access_byte, set_access_byte: 47, 40;
    limit_high, set_limit_high: 51, 48;
    flags, set_flags: 55, 52;
    base_high, set_base_high: 63, 56;
    base_high32, set_base_high32: 95, 64;
    rsv, _: 127, 96;
}

#[derive(Debug, Default)]
#[repr(C, packed)]
pub struct Tss {
    _rsv0: u32,
    rsp0: u64,
    rsp1: u64,
    rsp2: u64,
    rsv1: u32,
    rsv2: u32,
    ist1: u64,
    ist2: u64,
    ist3: u64,
    ist4: u64,
    ist5: u64,
    ist6: u64,
    ist7: u64,
    rsv3: u32,
    rsv4: u32,
    rsv5: u16,
    io_map_base_address: u16,
}

#[repr(C, packed)]
pub struct GdtDescriptor {
    size: u16,
    offset: usize,
}

#[repr(C)]
pub struct GlobalDescriptorTable {
    null_segment_descriptor: SegmentDescriptor,
    kernel_code: SegmentDescriptor,
    kernel_data: SegmentDescriptor,
    user_code: SegmentDescriptor,
    user_data: SegmentDescriptor,
    tss_descriptor: TssDescriptor,
}

impl GlobalDescriptorTable {
    pub fn init() {
        let gdtr = sgdt();
        let mut gdt = unsafe { &mut *(gdtr.offset as *mut GlobalDescriptorTable) };

        // This should already be set by the boot script, but I keep it as a reference
        gdt.kernel_code.set_access_byte(0b10011000); // E S P 43 44 47
        gdt.kernel_code.set_flags(0b0010); // L

        gdt.kernel_data.set_access_byte(0b10010110); // P S DW RW  41 42 44  47
        gdt.kernel_data.set_flags(0);

        gdt.user_code.set_access_byte(0b11111010); // P DPL S E RW 41 43 44 45 46 47
        gdt.user_code.set_flags(0b0010); // L

        gdt.user_data.set_access_byte(0b11110010); // P DPL S RW 41 44 45 46 47
        gdt.user_data.set_flags(0b0010); // L

        Self::setup_tss(gdt);
        Self::load_gdt(gdtr.offset);
    }

    fn setup_tss(gdt: &mut GlobalDescriptorTable) {
        // Question: Should this be updated everytime we jump in user mode?
        let rsp = crate::arch::x86_64::registers::rsp();

        let mut tss = Box::into_pin(Box::new(Tss::default()));
        tss.rsp0 = rsp as u64;
        tss.rsp1 = rsp as u64;
        tss.rsp2 = rsp as u64;

        let tss_address = &*tss as *const Tss as u128;
        gdt.tss_descriptor.set_limit_low(size_of::<Tss>() as u128); // maybe this should be size - 1
        gdt.tss_descriptor.set_base_low(tss_address & 0xFFFF);
        gdt.tss_descriptor.set_base_mid(tss_address >> 16 & 0xFF);
        gdt.tss_descriptor.set_access_byte(0b10001001);
        gdt.tss_descriptor.set_flags(0b1000);
        gdt.tss_descriptor.set_base_high(tss_address >> 24 & 0xFF);
        gdt.tss_descriptor.set_base_high32(tss_address >> 32 & 0xFFFFFFFF);
    }

    fn load_gdt(offset: usize) {
        // Update the GDT pointer
        let updated_gdtr = GdtDescriptor {
            size: size_of::<GlobalDescriptorTable>() as u16 - 1,
            offset,
        };

        unsafe {
            // Load GDT
            asm!("lgdt [{}]", in(reg) &updated_gdtr);

            // Flush TSS
            asm!("mov ax, 5 * 8", "ltr ax");
        }
    }
}


#[inline]
fn sgdt() -> GdtDescriptor {
    let mut gdtr: GdtDescriptor = GdtDescriptor {
        size: 0,
        offset: 0,
    };

    unsafe {
        asm!("sgdt [{}]", in(reg) &mut gdtr, options(readonly, nostack, preserves_flags));
    }

    gdtr
}

pub fn jump_to_user_mode() {
    unsafe {
        asm! {
            "mov ax, (4 * 8) | 3
            mov ds, ax
            mov es, ax
            mov fs, ax
            mov gs, ax

            mov eax, esp
            push (4 * 8) | 3
            push rax
            pushf
            push (3 * 8) | 3"
        }

        asm! {
            "push {}
            iretq",
            in(reg) test_user_function as usize,
        }
    }
}

extern "C" fn test_user_function() {
    println!("Welcome to user land!!");

    loop {}
}
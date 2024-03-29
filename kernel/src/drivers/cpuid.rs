use alloc::string::{String, ToString};
use core::str;
use core::arch::asm;
use conquer_once::spin::OnceCell;
use crate::utils::{any_as_u8_slice};
use crate::utils::bitutils::is_nth_bit_set;

static CPU_INFO: OnceCell<CPUInfo> = OnceCell::uninit();

struct CPUVendorResponse {
    ebx: u32,
    edx: u32,
    ecx: u32,
}

struct BrandStringResponse {
    eax: u32,
    ebx: u32,
    ecx: u32,
    edx: u32,
    eax2: u32,
    ebx2: u32,
    ecx2: u32,
    edx2: u32,
    eax3: u32,
    ebx3: u32,
    ecx3: u32,
    edx3: u32,
}

pub enum CPUVendor {
    Amd,
    Intel,
}

pub struct CPUInfo {
    vendor: CPUVendor,
    is_apic_supported: bool,
    brand_string: String,
}

impl CPUInfo {
    fn instance() -> Result<&'static CPUInfo, &'static str> {
        return match CPU_INFO.get() {
            Some(cpu) => Ok(cpu),
            None => {
                match CPU_INFO.try_init_once(|| Self::get_current_cpu_info()) {
                    Ok(_) => Ok(CPU_INFO.get().unwrap()),
                    Err(_) => Err("cpuid: cannot initialize CPU more than once")
                }
            }
        }

    }

    fn get_current_cpu_info() -> CPUInfo {
        info!("cpu: getting cpu info...");

        unsafe {
            Self {
                vendor: Self::get_vendor(),
                is_apic_supported: Self::get_apic_support(),
                brand_string: Self::get_brand_string(),
            }
        }
    }

    unsafe fn get_vendor() -> CPUVendor {
        let ebx: u32;
        let ecx: u32;
        let edx: u32;

        asm!("mov eax, 0x0; cpuid;");

        asm!("mov {:e}, ebx", out(reg) ebx, options(nomem, nostack, preserves_flags));
        asm!("mov {:e}, ecx", out(reg) ecx, options(nomem, nostack, preserves_flags));
        asm!("mov {:e}, edx", out(reg) edx, options(nomem, nostack, preserves_flags));

        let vendor_response = CPUVendorResponse { ebx, edx, ecx };
        let vendor_string = str::from_utf8(any_as_u8_slice(&vendor_response)).unwrap();

        match vendor_string {
            "AuthenticAMD" => CPUVendor::Amd,
            "GenuineIntel" => CPUVendor::Intel,
            _ => panic!("Unsupported CPU"),
        }
    }

    unsafe fn get_apic_support() -> bool {
        let edx: u32;

        asm!("mov eax, 0x1; cpuid;");
        asm!("mov {:e}, edx", out(reg) edx, options(nomem, nostack, preserves_flags));

        is_nth_bit_set(edx as usize, 9)
    }

    pub unsafe fn get_brand_string() -> String {
        let eax: u32;
        let ebx: u32;
        let ecx: u32;
        let edx: u32;

        asm!("mov eax, 0x80000002; cpuid;");
        asm!("mov {:e}, eax", out(reg) eax, options(nomem, nostack, preserves_flags));
        asm!("mov {:e}, ebx", out(reg) ebx, options(nomem, nostack, preserves_flags));
        asm!("mov {:e}, ecx", out(reg) ecx, options(nomem, nostack, preserves_flags));
        asm!("mov {:e}, edx", out(reg) edx, options(nomem, nostack, preserves_flags));

        let eax2: u32;
        let ebx2: u32;
        let ecx2: u32;
        let edx2: u32;

        asm!("mov eax, 0x80000003; cpuid;");
        asm!("mov {:e}, eax", out(reg) eax2, options(nomem, nostack, preserves_flags));
        asm!("mov {:e}, ebx", out(reg) ebx2, options(nomem, nostack, preserves_flags));
        asm!("mov {:e}, ecx", out(reg) ecx2, options(nomem, nostack, preserves_flags));
        asm!("mov {:e}, edx", out(reg) edx2, options(nomem, nostack, preserves_flags));

        let eax3: u32;
        let ebx3: u32;
        let ecx3: u32;
        let edx3: u32;

        asm!("mov eax, 0x80000004; cpuid;");
        asm!("mov {:e}, eax", out(reg) eax3, options(nomem, nostack, preserves_flags));
        asm!("mov {:e}, ebx", out(reg) ebx3, options(nomem, nostack, preserves_flags));
        asm!("mov {:e}, ecx", out(reg) ecx3, options(nomem, nostack, preserves_flags));
        asm!("mov {:e}, edx", out(reg) edx3, options(nomem, nostack, preserves_flags));

        let brand_response = BrandStringResponse { eax, ebx, ecx, edx, eax2, ebx2, ecx2, edx2, eax3, ebx3, ecx3, edx3, };
        str::from_utf8(any_as_u8_slice(&brand_response)).unwrap().to_string()
    }

    pub fn print_cpu_info() {
        let cpu_info = Self::instance();

        match cpu_info {
            Ok(cpu_info) => info!("{}", cpu_info.brand_string),
            Err(err) => error!("{}", err),
        }
    }
}
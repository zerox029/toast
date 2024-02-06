use crate::{println, print};

pub fn init_ps2_controller() {
    if !check_ps2_controller_exists() {
        println!("Could not find PS/2 controller...")
    }
}

pub fn check_ps2_controller_exists() -> bool {
    true
}
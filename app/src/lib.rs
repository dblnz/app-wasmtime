#![no_std]
#![no_main]

extern crate alloc;

// Phase 5: Implement main() that reads .cwasm from filesystem and runs it.

#[no_mangle]
pub extern "C" fn main() -> i32 {
    // TODO: Phase 5 — load .cwasm from initrd, call run_module / run_component
    0
}

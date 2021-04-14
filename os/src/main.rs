#![no_std]
#![no_main]
#![feature(global_asm)]
#![feature(llvm_asm)]
#![feature(panic_info_message)]


#[macro_use]
mod console;
mod task;
mod lang_items;
mod sbi;
mod syscall;
mod trap;
mod config;
mod loader;

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.S"));

use core::usize;

#[no_mangle]
pub fn rust_main() -> ! {
    console::init();
    clear_bss();
    println!("[kernel] Hello, world!");
    trap::init();
    loader::load_apps();
    task::run_first_task();
    panic!("Unreachable");
}

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| unsafe { (a as *mut u8).write_volatile(0) });
}

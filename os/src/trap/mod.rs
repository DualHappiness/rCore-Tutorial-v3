mod context;

use riscv::register::{mtvec::TrapMode, scause::{self, Exception, Trap}, sie, stval, stvec};
use timer::set_next_trigger;

use crate::{syscall::syscall, task::suspend_current_and_run_next, timer};

global_asm!(include_str!("trap.S"));

pub fn init() {
    extern "C" {
        fn __alltraps();
    }
    unsafe {
        stvec::write(__alltraps as usize, TrapMode::Direct);
    }
}

pub fn enable_timer_interrupt() {
    unsafe { sie::set_stimer(); }
}

#[no_mangle]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            println!("[kernel] PageFault in application, core dumped.");
            panic!("[kernel] Cannot continue!");
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction in application, core dumped.");
            panic!("[kernel] Cannot continue!");
        }
        Trap::Interrupt(scause::Interrupt::SupervisorTimer) => {
            set_next_trigger();
            suspend_current_and_run_next();
        }
        _ => panic!(
            "Unsupported trap {:?}, stval = {:#x}",
            scause.cause(),
            stval
        ),
    }
    cx
}

pub use context::TrapContext;

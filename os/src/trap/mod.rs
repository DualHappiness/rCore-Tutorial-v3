mod context;

use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Trap},
    sie, stval, stvec,
};
use timer::set_next_trigger;

use crate::{
    config::{TRAMPOLINE, TRAP_CONTEXT},
    syscall::syscall,
    task::{
        current_trap_cx, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next,
    },
    timer,
};

global_asm!(include_str!("trap.S"));

pub fn init() {
    set_kernel_trap_entry();
}

#[no_mangle]
pub fn trap_from_kernel() -> ! {
    panic!("a trap from kernel!");
}

fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(trap_from_kernel as usize, TrapMode::Direct);
    }
}

pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

#[no_mangle]
pub fn trap_handler() -> ! {
    set_kernel_trap_entry();
    let cx = current_trap_cx();
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::LoadPageFault) => {
            println!("[kernel] PageFault in application, core dumped.");
            exit_current_and_run_next();
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
    trap_return();
}

fn set_user_trap_entry() {
    unsafe {
        // va 实际指向最高页, 也就等于指向__alltraps
        stvec::write(TRAMPOLINE as usize, TrapMode::Direct);
    }
}

pub fn trap_return() -> ! {
    set_user_trap_entry();
    let trap_cx_prt = TRAP_CONTEXT;
    let user_satp = current_user_token();
    extern "C" {
        fn __alltraps();
        fn __restore();
    }
    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;
    unsafe {
        llvm_asm!("fence.i" :::: "volatile");
        llvm_asm!("jr $0" :: "r"(restore_va), "{a0}"(trap_cx_prt), "{a1}"(user_satp) :: "volatile");
    }

    panic!("Unreachable in back_to_user!");
}

pub use context::TrapContext;

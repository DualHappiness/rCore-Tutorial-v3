use crate::config::CLOCK_FREQ;
use crate::sbi::set_timer;
use riscv::register::time;

const TICKS_PER_SEC: usize = 100;
const MSEC_PER_SEC: usize = 1000;
const USEC_PER_SEC: usize = 1000_000;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

impl TimeVal {
    pub fn new() -> Self {
        TimeVal { sec: 0, usec: 0 }
    }
}

pub fn get_time() -> usize {
    time::read()
}

pub fn get_time_ms() -> TimeVal {
    let time = time::read();
    let sec = time / CLOCK_FREQ;
    let usec = (time - sec * CLOCK_FREQ) / (CLOCK_FREQ / USEC_PER_SEC);
    TimeVal { sec, usec }
}

pub fn set_next_trigger() {
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC);
}

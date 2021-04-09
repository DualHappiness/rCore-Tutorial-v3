use crate::*;
use core::fmt::{self, Write};
use log::{self, Level, LevelFilter, Log, Metadata, Record};

macro_rules! with_color {
    ($args: ident, $color: ident) => {{
        format_args!("\u{1B}[{}m{}\u{1B}[0m", $color as u8, $args)
    }};
}

const SYSCALL_WRITE: usize = 64;
const SBI_CONSOLE_PUTCHAR: usize = 1;

pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    syscall(SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}

pub fn console_putchar(c: usize) {
    syscall(SBI_CONSOLE_PUTCHAR, [c, 0, 0]);
}

struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        // sys_write(1, s.as_bytes());
        for c in s.chars() {
            console_putchar(c as usize);
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

pub fn print_with_color(args: fmt::Arguments, color: u8) {
    print(with_color!(args, color));
}

struct SimpleLogger;

impl Log for SimpleLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        print_with_color(
            format_args!("[{:>5}] {}", record.level(), record.args()),
            level_to_color(record.level()),
        );
    }

    fn flush(&self) {}
}

fn level_to_color(level: Level) -> u8 {
    match level {
        Level::Error => 31,
        Level::Warn => 93,
        Level::Info => 34,
        Level::Debug => 32,
        Level::Trace => 90,
    }
}

pub fn init() {
    static LOGGER: SimpleLogger = SimpleLogger;
    log::set_logger(&LOGGER).unwrap();
    // ! option_env! 是编译期获取
    log::set_max_level(match option_env!("LOG") {
        Some("error") => LevelFilter::Error,
        Some("warn") => LevelFilter::Warn,
        Some("info") => LevelFilter::Info,
        Some("debug") => LevelFilter::Debug,
        Some("trace") => LevelFilter::Trace,
        _ => LevelFilter::Off,
    });
}

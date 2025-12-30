use core::fmt::{self, Write};

struct Console;

impl Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        crate::drivers::uart::write_str(s);
        Ok(())
    }
}

pub fn _print(args: fmt::Arguments) {
    let _ = Console.write_fmt(args);
}

// Note: The print! and println! macros are now defined in lib.rs
// to avoid duplicate definitions and ensure they're available at crate root

// Public convenience functions - don't conflict with macro names
pub fn print_fmt(_args: core::fmt::Arguments) {
    _print(_args);
}

pub fn println_fmt(_args: core::fmt::Arguments) {
    _print(format_args!("{}\n", _args));
}

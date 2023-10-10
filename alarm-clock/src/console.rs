//! Universal `println` macro for writing to the serial connection

use crate::shared::UsbSerial;
use avr_device::interrupt::{self, Mutex};
use core::cell::RefCell;

pub static CONSOLE: Mutex<RefCell<Option<UsbSerial>>> = interrupt::Mutex::new(RefCell::new(None));

pub const DIGIT_LOOKUP: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F',
];

macro_rules! print {
    ($($t:tt)*) => {
        avr_device::interrupt::free(
            |critical_section| {
                if let Some(console) = crate::console::CONSOLE.borrow(critical_section).borrow_mut().as_mut() {
                    let _ = ufmt::uwrite!(console, $($t)*);
                }
            },
        )
    };
}

macro_rules! println {
    ($($t:tt)*) => {
        avr_device::interrupt::free(
            |critical_section| {
                arduino_hal::delay_ms(1);
                if let Some(console) = crate::console::CONSOLE.borrow(critical_section).borrow_mut().as_mut() {
                    let _ = ufmt::uwriteln!(console, $($t)*);
                };
            },
        )
    };
}

macro_rules! debug {
    ($($t:tt)*) => {
        // TODO: Call println! macro from here and compile time check
        if crate::shared::DEBUG {
            avr_device::interrupt::free(
                |critical_section| {
                    if let Some(console) = crate::console::CONSOLE.borrow(critical_section).borrow_mut().as_mut() {
                        let _ = ufmt::uwriteln!(console, $($t)*);
                    };
                }
            )
        }
    };
}

macro_rules! trace {
    ($($t:tt)*) => {
        // TODO: Call println! macro from here and compile time check
        if crate::shared::TRACE {
            avr_device::interrupt::free(
                |critical_section| {
                    if let Some(console) = crate::console::CONSOLE.borrow(critical_section).borrow_mut().as_mut() {
                        let _ = ufmt::uwriteln!(console, $($t)*);
                    };
                }
            )
        }
    }
}

pub fn set_console(console: UsbSerial) {
    interrupt::free(|cs| {
        *CONSOLE.borrow(cs).borrow_mut() = Some(console);
    })
}

pub(crate) use debug;
pub(crate) use print;
pub(crate) use println;
pub(crate) use trace;

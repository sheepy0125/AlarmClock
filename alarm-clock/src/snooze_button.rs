//! Snooze button! Like the rotary encoder, this really just interacts with
//! the interrupt handler. See `interrupts.rs` for more!

use crate::{
    console::{debug, println},
    interrupts::{changed_state, get_snooze_button_pressed, RotaryEncoderState},
    pins::{self, RotaryEncoderPins},
    shared::{
        PinState::{PinState, HIGH, LOW},
        TimeDigits,
    },
    shift_register::ShiftRegister,
};
use arduino_hal::{
    hal::port::{self, Dynamic},
    port::{
        mode::{Input, Output, PullUp},
        Pin,
    },
};
use avr_device::interrupt::CriticalSection;
use core::sync::atomic::AtomicBool;

pub struct SnoozeButton {
    pub button: pins::snooze::Button,
    state: bool,
    changed: bool,
}
impl SnoozeButton {
    pub fn new(button: pins::snooze::Button) -> Self {
        Self {
            button,
            state: false,
            changed: false,
        }
    }

    pub fn update(&mut self) {
        let state = get_snooze_button_pressed();
        // Only detect snooze button presses, ignore rotary encoder changes
        self.changed = changed_state() && (self.state != state);
        debug!(
            "[DEBUG] [SNOOZE] Snooze button update, changed: {}",
            match self.changed {
                true => "YES",
                false => "NO",
            }
        );
        self.state = state;
    }

    pub fn pressed(&mut self) -> bool {
        let ret = self.changed && self.state;
        self.changed = false;
        ret
    }
}

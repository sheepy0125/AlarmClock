//! Snooze button! Like the rotary encoder, this really just interacts with
//! the interrupt handler. See `interrupts.rs` for more!

use crate::{
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

    pub fn update<'cs>(&mut self, critical_section: &CriticalSection<'cs>) {
        let state = get_snooze_button_pressed(critical_section);
        // Only detect snooze button presses, ignore rotary encoder changes
        self.changed = changed_state(critical_section) && (self.state != state);
        self.state = state;
    }

    pub fn pressed(&mut self) -> bool {
        let ret = self.changed && self.state;
        self.changed = false;
        ret
    }
}

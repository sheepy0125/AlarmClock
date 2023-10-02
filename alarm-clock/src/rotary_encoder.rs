//! Rotary encoder for interfacing with the character LCD and setting aspects of
//! alarm clock; only the rotary encoder is implemented here!!
//! See the rotary encoder interrupt in `interrupts.rs` as that is where the encoder
//! state is set.

use crate::{
    interrupts::{changed_state, get_rotary_encoder_state, RotaryEncoderState},
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

pub struct RotaryEncoder {
    a: pins::rotary_encoder::A,
    b: pins::rotary_encoder::B,
    button: pins::rotary_encoder::Button,
    state: RotaryEncoderState,
    pub changed: bool,
}
impl RotaryEncoder {
    pub fn new(
        a: pins::rotary_encoder::A,
        b: pins::rotary_encoder::B,
        button: pins::rotary_encoder::Button,
    ) -> Self {
        Self {
            a,
            b,
            button,
            changed: false,
            state: RotaryEncoderState {
                a: false,
                b: false,
                button: false,
            },
        }
    }

    pub fn from_pins(rotary_encoder_pins: RotaryEncoderPins) -> Self {
        Self::new(
            rotary_encoder_pins.a,
            rotary_encoder_pins.b,
            rotary_encoder_pins.button,
        )
    }

    pub fn update<'cs>(&mut self, critical_section: &CriticalSection<'cs>) {
        let state = get_rotary_encoder_state(critical_section);
        // Only detect rotary encoder changes, ignore snooze press
        self.changed = changed_state(critical_section)
            && (self.state.a != state.a
                || self.state.b != state.b
                || self.state.button != state.button);
        self.state = state;
    }

    pub fn rotated_clockwise(&mut self) -> bool {
        let ret = self.changed && (self.state.a != self.state.b);
        self.changed = false;
        ret
    }

    pub fn button(&mut self) -> bool {
        let ret = self.changed && self.state.button;
        self.changed = false;
        ret
    }
}

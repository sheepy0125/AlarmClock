//! An arbitrarily-lengthed latching shift register that supports updating all
//! output drains at once, unlike the `shift-register-driver` crate (see issue #1
//! of their crate)

use core::{
    cell::RefCell,
    mem::{self, MaybeUninit},
    ptr,
    sync::atomic::{AtomicBool, Ordering::SeqCst},
};

use crate::{
    console::{debug, trace},
    pins::ShiftRegisterPins,
    shared::PinState::{PinState, HIGH, LOW},
};
use arduino_hal::{
    delay_us,
    hal::port::Dynamic,
    port::{mode::Output, Pin},
};
use avr_device::interrupt::{free as interrupt_free, CriticalSection};
use embedded_hal::digital::v2::OutputPin;

// Defined on page 4 of https://www.ti.com/lit/ds/symlink/tpic6595.pdf
// The serial input width is 20ns (Tsu + Th) so we don't need to explicitly account for it
const SERIAL_RISING_EDGE_PADDING_NS: u32 = 10_u32; // Tsu
const SERIAL_FALLING_EDGE_PADDING_NS: u32 = SERIAL_RISING_EDGE_PADDING_NS; // Th
const CLOCK_WIDTH_NS: u32 = 20_u32; // Tw
const LATCH_WIDTH_NS: u32 = CLOCK_WIDTH_NS;

/// Generic shift register with automatic software latching for every N bits
pub struct ShiftRegister<const N: usize, SerialInput, Clock, Latch>
where
    SerialInput: OutputPin,
    Clock: OutputPin,
    Latch: OutputPin,
{
    pub serial_input_pin: SerialInput,
    pub clock_pin: Clock,
    pub latch_pin: Latch,
    pub is_latched: bool,
    current_shifted_bit: usize,
    bit_array: [PinState; N],
}
impl<const N: usize, SerialInput, Clock, Latch> ShiftRegister<N, SerialInput, Clock, Latch>
where
    SerialInput: OutputPin,
    Clock: OutputPin,
    Latch: OutputPin,
{
    pub fn new(serial_input_pin: SerialInput, clock_pin: Clock, latch_pin: Latch) -> Self {
        Self {
            serial_input_pin,
            clock_pin,
            latch_pin,
            is_latched: false,
            bit_array: [LOW; N],
            current_shifted_bit: 0_usize,
        }
    }

    pub fn from_pins(shift_register_pins: ShiftRegisterPins<SerialInput, Clock, Latch>) -> Self {
        Self::new(
            shift_register_pins.serial_input,
            shift_register_pins.clock,
            shift_register_pins.latch,
        )
    }

    /// Shift a bit out. If the bit shifted is equal to N then it will latch.
    pub fn shift_out(&mut self, state: PinState, update_bit_array: bool) {
        // Rising edge of serial in pin (setup)
        if state {
            let _ = self.serial_input_pin.set_high();
        } else {
            let _ = self.serial_input_pin.set_low();
        }
        delay_us(SERIAL_RISING_EDGE_PADDING_NS);

        // Rising edge of clock pulse
        let _ = self.clock_pin.set_high();

        // Falling edge of serial in pin (tie to GND again)
        delay_us(SERIAL_FALLING_EDGE_PADDING_NS);
        let _ = self.serial_input_pin.set_low();

        // Falling edge of clock pulse
        delay_us(CLOCK_WIDTH_NS - SERIAL_FALLING_EDGE_PADDING_NS);
        let _ = self.clock_pin.set_low();

        // Latch if all bits shifted out
        let current = self.current_shifted_bit + 1;
        self.current_shifted_bit = current;
        if current == N {
            self.latch();
            return;
        }

        // No latching, update bit array
        if update_bit_array {
            self.bit_array.rotate_right(1_usize);
            self.bit_array[0] = state;
            self.is_latched = false
        }
    }

    /// Latch the shift register and reset the shift register
    pub fn latch(&mut self) {
        trace!("[TRACE] [SHIFT REG] Latching");
        self.is_latched = true;

        let _ = self.latch_pin.set_low();
        delay_us(LATCH_WIDTH_NS);
        let _ = self.latch_pin.set_high();

        self.current_shifted_bit = 0_usize;
    }

    /// Shift out a bit array so that the first output on the first shift
    /// register is equal to the first bit in the array
    pub fn set_bit_array(&mut self, bit_array: [PinState; N]) {
        for state in bit_array.iter().rev() {
            self.shift_out(*state, false);
        }
        self.bit_array = bit_array;
    }
}

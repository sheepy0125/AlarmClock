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

trait ShiftRegisterInternal {
    /// Update an index of the bit array and shift all bits. This won't reset
    /// the counter of whatever bit is currently shifted.
    fn update_idx(&self, pin_idx: usize, pin_state: PinState);
}

/// A fake pin that has a handle to the shift register, so that a shift register
/// can be used as normal pins. Upon every write to this pin, the shift register
/// latches. Please ensure the shift register never goes out of scope, as it's hard
/// to validate ordinary memory safety rules here; there is a shared shift register
/// object among all pins!
pub struct FakePin<'a> {
    shift_register: &'a dyn ShiftRegisterInternal,
    pub idx: usize,
}
impl<'a> FakePin<'a> {
    pub fn new(shift_register: &'a dyn ShiftRegisterInternal, idx: usize) -> Self {
        Self {
            shift_register,
            idx,
        }
    }
}
impl OutputPin for FakePin<'_> {
    type Error = ();
    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.shift_register.update_idx(self.idx, true);
        Ok(())
    }
    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.shift_register.update_idx(self.idx, false);
        Ok(())
    }
    // fn set_state(&mut self, state: embedded_hal::digital::v2::PinState) -> Result<(), Self::Error> {
    // self.shift_register.update_idx(self.idx, state);
    // Ok(())
    // }
}

/// TPIC6595 shift register with automatic software latching for every 8 bits
pub struct ShiftRegister<const N: usize, SerialInput, Clock, Latch>
where
    SerialInput: OutputPin,
    Clock: OutputPin,
    Latch: OutputPin,
{
    pub serial_input_pin: RefCell<SerialInput>,
    pub clock_pin: RefCell<Clock>,
    pub latch_pin: RefCell<Latch>,
    pub is_latched: AtomicBool,
    current_shifted_bit: RefCell<usize>,
    bit_array: RefCell<[PinState; N]>,
}
impl<const N: usize, SerialInput, Clock, Latch> ShiftRegister<N, SerialInput, Clock, Latch>
where
    SerialInput: OutputPin,
    Clock: OutputPin,
    Latch: OutputPin,
{
    pub fn new(serial_input_pin: SerialInput, clock_pin: Clock, latch_pin: Latch) -> Self {
        Self {
            serial_input_pin: RefCell::new(serial_input_pin),
            clock_pin: RefCell::new(clock_pin),
            latch_pin: RefCell::new(latch_pin),
            is_latched: AtomicBool::new(false),
            bit_array: RefCell::new([LOW; N]),
            current_shifted_bit: RefCell::new(0_usize),
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
    pub fn shift_out<'cs>(
        &self,
        state: PinState,
        _critical_section: &CriticalSection<'cs>,
        update_bit_array: bool,
    ) {
        // Rising edge of serial in pin (setup)
        if state {
            let _ = self.serial_input_pin.borrow_mut().set_high();
        } else {
            let _ = self.serial_input_pin.borrow_mut().set_low();
        }
        delay_us(SERIAL_RISING_EDGE_PADDING_NS);

        // Rising edge of clock pulse
        let _ = self.clock_pin.borrow_mut().set_high();

        // Falling edge of serial in pin (tie to GND again)
        delay_us(SERIAL_FALLING_EDGE_PADDING_NS);
        let _ = self.serial_input_pin.borrow_mut().set_low();

        // Falling edge of clock pulse
        delay_us(CLOCK_WIDTH_NS - SERIAL_FALLING_EDGE_PADDING_NS);
        let _ = self.clock_pin.borrow_mut().set_low();

        // Latch if all bits shifted out
        self.current_shifted_bit
            .replace(*self.current_shifted_bit.borrow() + 1);
        if *self.current_shifted_bit.borrow() == N {
            self.latch();
            return;
        }

        // No latching, update bit array
        if update_bit_array {
            self.bit_array.borrow_mut().rotate_right(1_usize);
            self.bit_array.borrow_mut()[0] = state;
            self.is_latched.store(false, SeqCst);
        }
    }

    /// Latch the shift register and reset the shift register
    pub fn latch(&self) {
        self.is_latched.store(true, SeqCst);

        let _ = self.latch_pin.borrow_mut().set_high();
        delay_us(LATCH_WIDTH_NS);
        let _ = self.latch_pin.borrow_mut().set_low();

        for state in self.bit_array.borrow_mut().iter_mut() {
            *state = LOW;
        }

        self.current_shifted_bit.replace(0_usize);
    }

    /// Shift out a bit array so that the first output on the first shift
    /// register is equal to the first bit in the array
    pub fn set_bit_array<'cs>(
        &mut self,
        bit_array: [PinState; N],
        critical_section: &CriticalSection<'cs>,
    ) {
        for state in bit_array.iter().rev() {
            self.shift_out(*state, critical_section, false);
        }
        self.bit_array.replace(bit_array);
    }

    /// Decompose the shift register into fake "pins" that update the shift
    /// register. Please note, these pins *always* latch the shift register!
    /// Read the documentation for `FakePin`.
    pub fn decompose(&mut self) -> [FakePin; N] {
        // This is usurped from the shift-register-driver crate!!

        let mut pins: [FakePin; N];

        unsafe {
            // Safety guarantees are wonky here, transmuting from MaybeUninit
            // claims that `FakePin` isn't sized! If it's not sized, then how
            // are we able to do this?! XXX
            pins = mem::uninitialized();
            for (idx, element) in pins[..].iter_mut().enumerate() {
                ptr::write(element, FakePin::new(self, idx));
            }
        }

        pins
    }
}

impl<const N: usize, SerialInput, Clock, Latch> ShiftRegisterInternal
    for ShiftRegister<N, SerialInput, Clock, Latch>
where
    SerialInput: OutputPin,
    Clock: OutputPin,
    Latch: OutputPin,
{
    fn update_idx(&self, pin_idx: usize, pin_state: PinState) {
        self.bit_array.borrow_mut()[pin_idx] = pin_state;
        let old_shifted_bit = self.current_shifted_bit.replace(0_usize);
        interrupt_free(|critical_section| {
            for state in self.bit_array.borrow().iter().rev() {
                self.shift_out(*state, &critical_section, false);
            }
        });
        self.current_shifted_bit.replace(old_shifted_bit);
        self.is_latched.store(false, SeqCst);
    }
}

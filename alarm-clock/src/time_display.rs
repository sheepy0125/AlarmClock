//! All time displays

use crate::{
    pins,
    shared::{
        PinState::{PinState, HIGH, LOW},
        TimeDigits,
    },
    shift_register::ShiftRegister,
};
use arduino_hal::{
    hal::port::Dynamic,
    port::{mode::Output, Pin},
};
use avr_device::interrupt::{free as interrupt_free, CriticalSection};
use core::cell::RefCell;

#[repr(u8)]
#[derive(Clone, Copy)]
enum FourDigit {
    Hour1 = 0b1000,
    Hour2 = 0b0100,
    Minute1 = 0b0010,
    Minute2 = 0b0001,
}

/// Trait that all seven time displays have
pub trait Display {
    fn display(&mut self);
}

/// A, B, C, D, E, F, G, & DP pin states for a given digit index
const SEVEN_SEGMENT_OUTPUT: [[PinState; 8]; 0x10 + 1] = [
    /* Decimal */
    [HIGH, HIGH, HIGH, HIGH, HIGH, HIGH, LOW, LOW], // 0
    [LOW, HIGH, HIGH, LOW, LOW, LOW, LOW, LOW],     // 1
    [HIGH, HIGH, LOW, HIGH, HIGH, LOW, HIGH, LOW],  // 2
    [HIGH, HIGH, HIGH, HIGH, LOW, LOW, HIGH, LOW],  // 3
    [LOW, HIGH, HIGH, LOW, LOW, HIGH, HIGH, LOW],   // 4
    [HIGH, LOW, HIGH, HIGH, LOW, HIGH, HIGH, LOW],  // 5
    [HIGH, LOW, HIGH, HIGH, HIGH, HIGH, HIGH, LOW], // 6
    [HIGH, HIGH, HIGH, LOW, LOW, LOW, LOW, LOW],    // 7
    [HIGH, HIGH, HIGH, HIGH, HIGH, HIGH, HIGH, LOW], // 8
    [HIGH, HIGH, HIGH, HIGH, LOW, HIGH, HIGH, LOW], // 9
    /* (Scuffed) hexadecimal, where B == 8 and D == 0 */
    /* Hexadecimal is denoted by having the DP state on */
    [HIGH, HIGH, HIGH, LOW, HIGH, HIGH, HIGH, HIGH], // A
    [HIGH, HIGH, HIGH, HIGH, HIGH, HIGH, HIGH, HIGH], // B
    [HIGH, LOW, LOW, HIGH, HIGH, HIGH, LOW, HIGH],   // C
    [HIGH, HIGH, HIGH, HIGH, HIGH, HIGH, LOW, HIGH], // D
    [HIGH, LOW, LOW, HIGH, HIGH, HIGH, HIGH, HIGH],  // E
    [HIGH, LOW, LOW, LOW, HIGH, HIGH, HIGH, HIGH],   // F
    /* This is just here to reset to an off state */
    [LOW, LOW, LOW, LOW, LOW, LOW, LOW, LOW], // NULL
];

/// 4-digit 7-segment display for hours and minutes
///
/// This is the 1.2" KW4-12041CUYA display in yellow.
/// 12 pins are needed to drive the display, split into 2-8-bit shift registers.
/// The first 8 pins on the first shift register are used to control A-G and DP,
/// while the beginning 4 pins on the second shift register are used to denote
/// which 7-segment digit is lit.
/// Every 5 milliseconds or so, the shift registers should be updated to the next
/// digit to produce a "always on" effect.
pub struct HoursMinutes<'a> {
    shift_register: ShiftRegister<
        12_usize,
        pins::hours_minutes_display::SerialIn,
        pins::hours_minutes_display::Clock,
        pins::hours_minutes_display::Latch,
    >,
    selected_digit: FourDigit,
    current_time: &'a RefCell<TimeDigits>,
}

impl<'a> HoursMinutes<'a> {
    pub fn new(
        shift_register: ShiftRegister<
            12_usize,
            pins::hours_minutes_display::SerialIn,
            pins::hours_minutes_display::Clock,
            pins::hours_minutes_display::Latch,
        >,
        time_ref: &'a RefCell<TimeDigits>,
    ) -> Self {
        Self {
            shift_register,
            selected_digit: FourDigit::Hour1,
            current_time: time_ref,
        }
    }
}
impl<'a> Display for HoursMinutes<'a> {
    /// Display and update loop. This should be called once every 5 milliseconds
    /// to ensure that all digits appear lit at the same time.
    fn display(&mut self) {
        let mut bitwise_digit = self.selected_digit as u8;

        // First 4 bits shifted in; former nybl of second shift register
        let selected_digit_pin_states: [PinState; 4_usize] = [
            (bitwise_digit >> 3) != 0,
            (bitwise_digit >> 2) != 0,
            (bitwise_digit >> 1) != 0,
            (bitwise_digit >> 0) != 0,
        ];

        // Last 8 bits shifted in; all outputs of first shift register
        let time_ref = self.current_time.borrow();
        let digit = (time_ref.hours.0 * bitwise_digit >> 3)
            + (time_ref.hours.1 * bitwise_digit >> 2)
            + (time_ref.minutes.0 * bitwise_digit >> 1)
            + (time_ref.minutes.1 * bitwise_digit >> 0);
        let digit_pin_states = SEVEN_SEGMENT_OUTPUT[digit as usize];

        let mut pin_states: [PinState; 12] = [LOW; 12_usize];
        pin_states[0..8].copy_from_slice(&digit_pin_states[..]);
        pin_states[8..12].copy_from_slice(&selected_digit_pin_states[..]);

        // Shift!
        interrupt_free(|critical_section| {
            self.shift_register
                .set_bit_array(pin_states, &critical_section);
        });
        // Assume the shift register is latched as this is the only time we update it

        // Rotate digit right for next display
        // Saftey: Bitwise digit will only be in one of the possible states of FourDigit
        bitwise_digit >>= 1; // Shift to next
        bitwise_digit |= (bitwise_digit << 3) & 0b1111; // Rotate right
        self.selected_digit = unsafe { core::mem::transmute(bitwise_digit) };
    }
}

/// The seconds are just 2 10016AD seven segment digits in 2-8-bit shift registers
/// as 16 pins are needed to drive them. The first "second" digit is in the first
/// shift register and the second "second" digit is in the next.
/// Each shift register's outputs are ordered from A-G and then another pin for DP
pub struct Seconds<'a> {
    shift_register: ShiftRegister<
        { 2 * 8_usize },
        pins::seconds_display::SerialIn,
        pins::seconds_display::Clock,
        pins::seconds_display::Latch,
    >,
    current_time: &'a RefCell<TimeDigits>,
}
impl<'a> Seconds<'a> {
    pub fn new(
        shift_register: ShiftRegister<
            { 2 * 8_usize },
            pins::seconds_display::SerialIn,
            pins::seconds_display::Clock,
            pins::seconds_display::Latch,
        >,
        time_ref: &'a RefCell<TimeDigits>,
    ) -> Self {
        Self {
            shift_register,
            current_time: time_ref,
        }
    }
}
impl<'a> Display for Seconds<'a> {
    /// This should be called only once per second as the digits will remain
    /// illuminated (no need for "animations" to occur)
    fn display(&mut self) {
        let mut pin_states: [PinState; 16] = [LOW; 16_usize];
        let time_ref = self.current_time.borrow();
        pin_states[0..8].copy_from_slice(&SEVEN_SEGMENT_OUTPUT[time_ref.seconds.0 as usize][..]);
        pin_states[8..16].copy_from_slice(&SEVEN_SEGMENT_OUTPUT[time_ref.seconds.1 as usize][..]);

        // Shift!
        interrupt_free(|critical_section| {
            self.shift_register
                .set_bit_array(pin_states, &critical_section)
        });
        // Assume latching as, again, we are the only producer to these shift registers!
    }
}

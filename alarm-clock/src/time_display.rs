//! All time displays

use crate::{
    console::DIGIT_LOOKUP,
    console::{debug, println, trace},
    pins,
    shared::{
        PinState::{PinState, HIGH, LOW},
        TimeDigits,
    },
    shift_register::ShiftRegister,
    state::State,
};
use arduino_hal::{
    hal::port::Dynamic,
    port::{mode::Output, Pin},
};
use avr_device::interrupt::{free as interrupt_free, CriticalSection, Mutex};
use core::cell::{Cell, RefCell};

/// Allow access to the millisecond interrupt
pub static HOUR_MINUTE_DISPLAY: Mutex<RefCell<Option<HoursMinutes>>> =
    Mutex::new(RefCell::new(None));
// The default digits value should never be used
pub static DIGITS: Mutex<RefCell<TimeDigits>> = Mutex::new(RefCell::new(TimeDigits {
    hours: (0_u8, 0_u8),
    minutes: (0_u8, 0_u8),
    seconds: (0_u8, 0_u8),
}));

#[repr(u8)]
#[derive(Clone, Copy)]
enum DigitSelect {
    DP = 1_u8 << 0,
    Hour1 = 1_u8 << 1,
    Hour2 = 1_u8 << 2,
    Minute1 = 1_u8 << 3,
    Minute2 = 1_u8 << 4,
}

/// Trait that all seven time displays have
pub trait Display {
    fn display(&mut self, state: &State);
}

/// A, B, C, D, E, F, G, & DP pin states for a given digit index
const SEVEN_SEGMENT_OUTPUT: [[PinState; 7]; 0x10 + 1] = [
    /* Decimal */
    [HIGH, HIGH, HIGH, HIGH, HIGH, HIGH, LOW],  // 0
    [LOW, HIGH, HIGH, LOW, LOW, LOW, LOW],      // 1
    [HIGH, HIGH, LOW, HIGH, HIGH, LOW, HIGH],   // 2
    [HIGH, HIGH, HIGH, HIGH, LOW, LOW, HIGH],   // 3
    [LOW, HIGH, HIGH, LOW, LOW, HIGH, HIGH],    // 4
    [HIGH, LOW, HIGH, HIGH, LOW, HIGH, HIGH],   // 5
    [HIGH, LOW, HIGH, HIGH, HIGH, HIGH, HIGH],  // 6
    [HIGH, HIGH, HIGH, LOW, LOW, LOW, LOW],     // 7
    [HIGH, HIGH, HIGH, HIGH, HIGH, HIGH, HIGH], // 8
    [HIGH, HIGH, HIGH, HIGH, LOW, HIGH, HIGH],  // 9
    /* (Scuffed) hexadecimal, where B == 8 and D == 0 */
    [HIGH, HIGH, HIGH, LOW, HIGH, HIGH, HIGH],  // A
    [HIGH, HIGH, HIGH, HIGH, HIGH, HIGH, HIGH], // B
    [HIGH, LOW, LOW, HIGH, HIGH, HIGH, LOW],    // C
    [HIGH, HIGH, HIGH, HIGH, HIGH, HIGH, LOW],  // D
    [HIGH, LOW, LOW, HIGH, HIGH, HIGH, HIGH],   // E
    [HIGH, LOW, LOW, LOW, HIGH, HIGH, HIGH],    // F
    /* This is just here to reset to an off state */
    [LOW, LOW, LOW, LOW, LOW, LOW, LOW], // NULL
];

/// 4-digit 7-segment display for hours and minutes
///
/// This is the 1.2" KW4-12041CUYA display in yellow.
/// Every millisecond or so, the shift registers should be updated to the next
/// digit to produce a "always on" effect.
///
/// The digits are obtained through the global DIGITS mutex, and this should be
/// stored in the global HOURS_MINUTES mutex.
pub struct HoursMinutes {
    shift_register: ShiftRegister<
        16_usize,
        pins::hours_minutes_display::SerialIn,
        pins::hours_minutes_display::Clock,
        pins::hours_minutes_display::Latch,
    >,
    selected_digit: DigitSelect,
    last_digit: TimeDigits,
}

impl HoursMinutes {
    pub fn new(
        shift_register: ShiftRegister<
            16_usize,
            pins::hours_minutes_display::SerialIn,
            pins::hours_minutes_display::Clock,
            pins::hours_minutes_display::Latch,
        >,
    ) -> Self {
        Self {
            shift_register,
            selected_digit: DigitSelect::DP,
            last_digit: TimeDigits::default(),
        }
    }

    /// Display and update loop. This should be called once every millisecond
    /// to ensure that all digits appear lit at the same time.
    pub fn display<'cs>(&mut self, critical_section: CriticalSection<'cs>) {
        let bitwise_digit = self.selected_digit as u8;

        // First 5 bits of first shift register (mode)
        let selected_digit_pin_states: [PinState; 5] = [
            (bitwise_digit & (DigitSelect::DP as u8)) != 0,
            (bitwise_digit & (DigitSelect::Hour1 as u8)) != 0,
            (bitwise_digit & (DigitSelect::Hour2 as u8)) != 0,
            (bitwise_digit & (DigitSelect::Minute1 as u8)) != 0,
            (bitwise_digit & (DigitSelect::Minute2 as u8)) != 0,
        ];

        // Last 3 pins of first shift register (DP 1, 2, 3 & 4) and the last pin
        // of the second shift register (DP 5)
        let dp_pin_states: [PinState; 4] = match self.selected_digit {
            DigitSelect::DP => [
                LOW,  // DP 1 (colon 1 top)
                LOW,  // DP 2 (colon 1 bottom)
                HIGH, // DP 3 & 4 (colon 2)
                LOW,  // DP 5 (random decimal point for fun :^)!)
            ],
            _ => [LOW; 4],
        };

        // First 7 pins of second shift register
        let segment_pin_states = match self.selected_digit {
            DigitSelect::DP => &[LOW; 7],
            _ => {
                let (hours, minutes) = DIGITS
                    .borrow(critical_section)
                    .try_borrow()
                    .ok()
                    .map(|digits| (digits.hours, digits.minutes))
                    .unwrap_or((self.last_digit.hours, self.last_digit.minutes));

                let digit = hours.0 * (bitwise_digit >> 1 & 1)
                    + hours.1 * (bitwise_digit >> 2 & 1)
                    + minutes.0 * (bitwise_digit >> 3 & 1)
                    + minutes.1 * (bitwise_digit >> 4 & 1);

                self.last_digit.hours = hours;
                self.last_digit.minutes = minutes;

                &SEVEN_SEGMENT_OUTPUT[digit as usize]
            }
        };

        // {dig_dp, dig_1, dig_2, dig_3, dig_4,  // Common select
        //  dp_3_4, dp_2, dp_1, dp_5,            // Decimal points
        //  f, g, a, b, c, d, e}                 // Seven segment
        // Since this display is common cathode, the common selected should be GND
        let pin_states: [PinState; 16] = [
            !selected_digit_pin_states[0],
            !selected_digit_pin_states[1],
            !selected_digit_pin_states[2],
            !selected_digit_pin_states[3],
            !selected_digit_pin_states[4],
            dp_pin_states[0],
            dp_pin_states[1],
            dp_pin_states[2],
            dp_pin_states[3],
            segment_pin_states[5], // F
            segment_pin_states[6], // G
            segment_pin_states[0], // A
            segment_pin_states[1], // B
            segment_pin_states[2], // C
            segment_pin_states[3], // D
            segment_pin_states[4], // E
        ];

        // Shift!
        self.shift_register.set_bit_array(pin_states);
        // Assume the shift register is latched as this is the only time we update it

        // Rotate digit right for next display
        // Saftey: Bitwise digit will only be in one of the possible states of FourDigit
        let mut new_bitwise_digit = bitwise_digit >> 1;
        new_bitwise_digit |= (bitwise_digit << 4) & 0b11111; // Rotate right
        self.selected_digit = unsafe { core::mem::transmute(new_bitwise_digit) };
    }
}

/// The seconds are just 2 10016AD seven segment digits in 2-8-bit shift registers
/// as 16 pins are needed to drive them. The first "second" digit is in the first
/// shift register and the second "second" digit is in the next.
/// Each shift register's outputs are ordered from A-G and then another pin for DP
pub struct Seconds {
    shift_register: ShiftRegister<
        { 2 * 8_usize },
        pins::seconds_display::SerialIn,
        pins::seconds_display::Clock,
        pins::seconds_display::Latch,
    >,
}
impl Seconds {
    pub fn new(
        shift_register: ShiftRegister<
            { 2 * 8_usize },
            pins::seconds_display::SerialIn,
            pins::seconds_display::Clock,
            pins::seconds_display::Latch,
        >,
    ) -> Self {
        Self { shift_register }
    }
}
impl Display for Seconds {
    /// This should be called only once per second as the digits will remain
    /// illuminated (no need for "animations" to occur)
    fn display(&mut self, state: &State) {
        debug!(
            "[DEBUG] [:SS] Displaying {}{}",
            state.digits.seconds.0, state.digits.seconds.1
        );

        let digit_1_output = &SEVEN_SEGMENT_OUTPUT[state.digits.seconds.0 as usize];
        let digit_2_output = &SEVEN_SEGMENT_OUTPUT[state.digits.seconds.1 as usize];
        let pin_states: [PinState; 16] = [
            // Digit 1
            digit_1_output[6], // G
            digit_1_output[5], // F
            digit_1_output[0], // A
            digit_1_output[1], // B
            digit_1_output[4], // E
            digit_1_output[3], // D
            digit_1_output[2], // C
            LOW,               // DP
            digit_2_output[6], // G
            digit_2_output[5], // F
            digit_2_output[0], // A
            digit_2_output[1], // B
            LOW,               // DP
            digit_2_output[3], // D
            digit_2_output[2], // C
            digit_2_output[4], // E
        ];

        // Shift!
        self.shift_register.set_bit_array(pin_states);
        // Assume latching as, again, we are the only producer to these shift registers!
    }
}

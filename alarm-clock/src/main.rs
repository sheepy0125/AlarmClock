#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
#![allow(incomplete_features)]
#![feature(adt_const_params)]
#![feature(trait_alias)]
#![feature(stmt_expr_attributes)]

use ag_lcd::{Display, LcdDisplay, Lines};
use arduino_hal::default_serial;
use avr_device::{atmega328p::exint::pcicr::PCICR_SPEC, generic::Reg, interrupt};
use core::{cell::RefCell, marker::PhantomData};
use embedded_hal::digital::v2::OutputPin;
use pcf8523::Pcf8523;
use pins::{RotaryEncoderPins, ShiftRegisterPins};
use rotary_encoder::RotaryEncoder;
use shared::TimeDigits;
use shift_register::ShiftRegister;
use snooze_button::SnoozeButton;
use time_display::{HoursMinutes, Seconds};

pub mod interrupts;
pub mod panic;
pub mod pins;
mod rotary_encoder;
pub mod shared;
pub mod shift_register;
mod snooze_button;
mod time_display;

#[arduino_hal::entry]
fn main() -> ! {
    let peripherals = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(peripherals);
    let _serial = default_serial!(peripherals, pins, shared::BAUD_RATE);

    // Set up pin handles
    let hours_minute_display_shift_register_pins = ShiftRegisterPins {
        serial_input: pins.d2.into_output() as pins::hours_minutes_display::SerialIn,
        clock: pins.d3.into_output() as pins::hours_minutes_display::Clock,
        latch: pins.d4.into_output() as pins::hours_minutes_display::Latch,
    };
    let seconds_display_shift_register_pins = ShiftRegisterPins {
        serial_input: pins.d5.into_output() as pins::seconds_display::SerialIn,
        clock: pins.d6.into_output() as pins::seconds_display::Clock,
        latch: pins.d7.into_output() as pins::seconds_display::Latch,
    };
    let rotary_encoder_pins = RotaryEncoderPins {
        a: pins.d13.into_pull_up_input(),
        b: pins.a0.into_pull_up_input(),
        button: pins.d12.into_pull_up_input(),
    };
    let character_lcd_shift_register_pins = ShiftRegisterPins {
        serial_input: pins.d8.into_output() as pins::character_lcd::SerialIn,
        clock: pins.d9.into_output() as pins::character_lcd::Clock,
        latch: pins.d10.into_output() as pins::character_lcd::Latch,
    };
    let snooze_button_pin = pins.d11.into_pull_up_input() as pins::snooze::Button;
    let mut buzzer_pin = {
        let mut pin = pins.a5.into_output() as pins::buzzer::Buzzer;
        pin.set_low();
        pin
    };
    let mut alarm_led_pin = pins.a1.into_output() as pins::leds::Alarm;
    let mut pm_led_pin = pins.a2.into_output() as pins::leds::PM;

    // Intialize interrupts
    interrupts::millis_init(peripherals.TC0);
    unsafe {
        interrupts::rotary_encoder_init(
            &peripherals.EXINT.pcicr,
            &peripherals.EXINT.pcmsk0,
            &rotary_encoder_pins.a,
            &rotary_encoder_pins.button,
        );
        interrupts::snooze_button_init(
            &peripherals.EXINT.pcicr,
            &peripherals.EXINT.pcmsk0,
            &snooze_button_pin,
        );
    };
    unsafe { avr_device::interrupt::enable() };

    // Time initialization
    let mut time = RefCell::new(TimeDigits::default());

    // Display initialization
    let mut hours_minutes_display = HoursMinutes::new(
        ShiftRegister::<12_usize, _, _, _>::from_pins(hours_minute_display_shift_register_pins),
        &time,
    );
    let mut seconds_display = Seconds::new(
        ShiftRegister::<{ 2 * 8_usize }, _, _, _>::from_pins(seconds_display_shift_register_pins),
        &time,
    );
    let (mut character_lcd, mut _shift_register): LcdDisplay<_, _> = {
        let mut character_lcd_shift_register =
            ShiftRegister::<8_usize, _, _, _>::from_pins(character_lcd_shift_register_pins);
        let character_lcd_pins = character_lcd_shift_register.decompose();
        (
            match character_lcd_pins {
                // Refer to KiCad schematic for pin layout
                [_, rs, _, enabled, db4, db5, db6, db7] => {
                    LcdDisplay::new(rs, enabled, arduino_hal::Delay::new())
                        .with_half_bus(db4, db5, db6, db7)
                        .with_display(Display::On)
                        .with_lines(Lines::TwoLines)
                        .with_reliable_init(10_000_u16)
                        .build()
                }
            },
            character_lcd_shift_register,
        )
    };

    // Controls initialization
    let mut rotary_encoder = RotaryEncoder::from_pins(rotary_encoder_pins);
    let mut snooze_button = SnoozeButton::new(snooze_button_pin);

    loop {}
}

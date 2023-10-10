#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
#![allow(incomplete_features)]
#![feature(adt_const_params)]
#![feature(trait_alias)]
#![feature(stmt_expr_attributes)]

use ag_lcd::{Blink, Cursor, Display as LcdDisplayMode, LcdDisplay, Lines};
use arduino_hal::{default_serial, delay_ms, delay_us, prelude::_void_ResultVoidExt, Delay, I2c};
use avr_device::{atmega328p::exint::pcicr::PCICR_SPEC, generic::Reg, interrupt};
use console::{println, set_console};
use core::{cell::RefCell, fmt::Write, marker::PhantomData};
use embedded_hal::digital::v2::OutputPin;
use heapless::String;
use pins::{RotaryEncoderPins, ShiftRegisterPins};
use rotary_encoder::RotaryEncoder;
use rtc::RTC;
use shared::{Time, TimeDigits, UsbSerial};
use shift_register::ShiftRegister;
use shift_register_driver::sipo::ShiftRegister8 as DecomposableShiftRegister;
use snooze_button::SnoozeButton;
use state::{DateSetState, Menu, OperationalMode, State, StateLogic, TimeSetState};
use time_display::{Display as TimeDisplayTrait, HoursMinutes, Seconds};
use ufmt::uwriteln;

use crate::{
    console::debug,
    interrupts::millis,
    shared::{MILLIS_OVERFLOW_UPDATE_MARGIN, UPDATE_DELTATIME},
    time_display::{DIGITS, HOUR_MINUTE_DISPLAY},
};

pub mod console;
pub mod interrupts;
pub mod panic;
pub mod pins;
mod rotary_encoder;
mod rtc;
pub mod shared;
pub mod shift_register;
mod snooze_button;
pub mod state;
mod time_display;

#[arduino_hal::entry]
fn main() -> ! {
    let peripherals = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(peripherals);
    let mut serial: UsbSerial = default_serial!(peripherals, pins, shared::BAUD_RATE);
    set_console(serial);

    let mut state = State::new();

    println!("Hello from the Alarm Clock!");

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
        let mut pin = pins.a3.into_output() as pins::buzzer::Buzzer;
        pin.set_low();
        pin
    };
    let mut alarm_led_pin = pins.a1.into_output() as pins::leds::Alarm;
    let mut pm_led_pin = pins.a2.into_output() as pins::leds::PM;
    let mut iic_pins = pins::IICPins {
        sda: pins.a4.into_pull_up_input(),
        scl: pins.a5.into_pull_up_input(),
    };

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
    debug!("[DEBUG] I2C & RTC initialization");
    let mut i2c = I2c::new(peripherals.TWI, iic_pins.sda, iic_pins.scl, 1);
    let mut rtc = RTC::new(i2c);

    // Display initialization
    debug!("[DEBUG] Hours & minutes display initialization");

    // Plop the hours minute display to the global so the interrupt handler can access it
    interrupt::free(|critical_section| {
        let hours_minutes_display = HoursMinutes::new(
            ShiftRegister::<16_usize, _, _, _>::from_pins(hours_minute_display_shift_register_pins),
        );
        HOUR_MINUTE_DISPLAY
            .borrow(critical_section)
            .replace(Some(hours_minutes_display));
    });
    debug!("[DEBUG] Seconds display initialization");
    let mut seconds_display = Seconds::new(ShiftRegister::<{ 2 * 8_usize }, _, _, _>::from_pins(
        seconds_display_shift_register_pins,
    ));
    debug!("[DEBUG] Character LCD shift register initialization");
    let mut character_lcd_shift_register = DecomposableShiftRegister::new(
        character_lcd_shift_register_pins.clock,
        character_lcd_shift_register_pins.latch,
        character_lcd_shift_register_pins.serial_input,
    );
    let mut character_lcd_pins = character_lcd_shift_register.decompose();
    let mut character_lcd: LcdDisplay<_, _> = match character_lcd_pins {
        // Refer to KiCad schematic for pin layout
        [_, rs, _, enabled, db4, db5, db6, db7] => {
            LcdDisplay::new(rs, enabled, arduino_hal::Delay::new())
                .with_half_bus(db4, db5, db6, db7)
                .with_display(LcdDisplayMode::On)
                .with_cursor(Cursor::Off)
                .with_lines(Lines::TwoLines)
                .build()
        }
    };

    // Controls initialization
    debug!("[DEBUG] Rotary encoder initialization");
    let mut rotary_encoder = RotaryEncoder::from_pins(rotary_encoder_pins);
    debug!("[DEBUG] Snooze button initialization");
    let mut snooze_button = SnoozeButton::new(snooze_button_pin);

    interrupt::free(|critical_section| {
        rtc.set_time(&state.time, &critical_section);
    });

    // Main loop
    loop {
        delay_ms(UPDATE_DELTATIME);
        debug!("[DEBUG] Loop iteration");

        // Update time
        if let Some(new_time) = rtc.read_time(&mut state.digits) {
            state.time = new_time;
        }
        interrupt::free(|critical_section| {
            DIGITS
                .borrow(critical_section)
                .replace(state.digits.clone());
        });

        character_lcd.clear();
        character_lcd.set_position(0, 0);
        delay_us(100_u32);
        character_lcd.print("alarmed clock");
        delay_us(100_u32);
        character_lcd.set_position(0, 1);
        delay_us(100_u32);
        let mut buf = [0_u8; 4];
        character_lcd.print(
            char::from_digit(state.digits.hours.0 as u32, 10_u32)
                .unwrap()
                .encode_utf8(&mut buf),
        );
        character_lcd.print(
            char::from_digit(state.digits.hours.1 as u32, 10_u32)
                .unwrap()
                .encode_utf8(&mut buf),
        );
        character_lcd.print(":");
        character_lcd.print(
            char::from_digit(state.digits.minutes.0 as u32, 10_u32)
                .unwrap()
                .encode_utf8(&mut buf),
        );
        character_lcd.print(
            char::from_digit(state.digits.minutes.1 as u32, 10_u32)
                .unwrap()
                .encode_utf8(&mut buf),
        );
        character_lcd.print(":");
        character_lcd.print(
            char::from_digit(state.digits.seconds.0 as u32, 10_u32)
                .unwrap()
                .encode_utf8(&mut buf),
        );
        character_lcd.print(
            char::from_digit(state.digits.seconds.1 as u32, 10_u32)
                .unwrap()
                .encode_utf8(&mut buf),
        );

        rotary_encoder.update();
        snooze_button.update();
        seconds_display.display(&state);
    }
}

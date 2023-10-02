//! Interrupts

use arduino_hal::{pac::TC0, pins, Peripherals};
use avr_device::{
    atmega328p::exint::{pcicr::PCICR_SPEC, pcmsk0::PCMSK0_SPEC},
    generic::Reg,
    interrupt::{self, Mutex},
};
use core::{
    cell::Cell,
    sync::atomic::{AtomicBool, Ordering::SeqCst},
};

use crate::{pins, shared::PinState::PinState};

pub use millis::{millis, millis_init};
pub use rotary_encoder_and_snooze::{
    changed_state, get_rotary_encoder_state, get_snooze_button_pressed, rotary_encoder_init,
    snooze_button_init, RotaryEncoderState,
};

/// This millisecond interrupt was usurped from Rahix's amazing blog:
/// https://blog.rahix.de/005-avr-hal-millis/
mod millis {
    use super::*;

    const PRESCALER: u32 = 1_024_u32;
    const TIMER_COUNTS: u32 = 125_u32;
    const MILLIS_INCREMENT: u32 = PRESCALER * TIMER_COUNTS / 16_000_u32; // Uno runs at 16MHz

    static MILLIS_COUNTER: Mutex<Cell<u32>> = Mutex::new(Cell::new(0_u32));

    pub fn millis_init(tc0: TC0) {
        // Configure the timer for the above interval (in CTC mode)
        // and enable its interrupt
        tc0.tccr0a.write(|w| w.wgm0().ctc());
        tc0.ocr0a.write(|w| w.bits(TIMER_COUNTS as u8));
        tc0.tccr0b.write(|w| match PRESCALER {
            8_u32 => w.cs0().prescale_8(),
            64_u32 => w.cs0().prescale_64(),
            256_u32 => w.cs0().prescale_256(),
            1024_u32 => w.cs0().prescale_1024(),
            _ => panic!(),
        });
        tc0.timsk0.write(|w| w.ocie0a().set_bit());

        // Reset the global millisecond counter
        interrupt::free(|critical_section| {
            MILLIS_COUNTER.borrow(critical_section).set(0_u32);
        });
    }

    #[avr_device::interrupt(atmega328p)]
    #[allow(non_snake_case)]
    fn TIMER0_COMPA() {
        interrupt::free(|critical_section| {
            let counter_cell = MILLIS_COUNTER.borrow(critical_section);
            let counter = counter_cell.get();
            counter_cell.set(counter + MILLIS_INCREMENT);
        })
    }

    /// Milliseconds since the interrupt timer was configured for all times that interrupts were allowed
    pub fn millis() -> u32 {
        interrupt::free(|critical_section| MILLIS_COUNTER.borrow(critical_section).get())
    }
}

mod rotary_encoder_and_snooze {
    use avr_device::interrupt::CriticalSection;

    use super::*;

    /// Set true for every interrupt
    static CHANGED_STATE: AtomicBool = AtomicBool::new(false);
    /// Whether the snooze button is pressed (tied to GND)
    static SNOOZE_BUTTON: AtomicBool = AtomicBool::new(false);
    static ROTARY_PIN_A: AtomicBool = AtomicBool::new(false);
    static ROTARY_PIN_B: AtomicBool = AtomicBool::new(false);
    /// Whether the rotary button is pressed (tied to GND)
    static ROTARY_BUTTON: AtomicBool = AtomicBool::new(false);

    /// Safety note: The caller must ensure that the A pin and Button pin are
    /// pin change interrupts 4 and 5 respectively of mask 0!
    pub unsafe fn rotary_encoder_init(
        pcicr: &Reg<PCICR_SPEC>,
        pcmsk0: &Reg<PCMSK0_SPEC>,
        _a: &pins::rotary_encoder::A,
        _button: &pins::rotary_encoder::Button,
    ) {
        // Enable mask 0 interrupt
        let mut enabled_interrupts = pcicr.read().bits() as u8;
        enabled_interrupts |= 0b1 << 0;
        pcicr.write(|w| unsafe { w.bits(enabled_interrupts) });

        // Configure mask 0
        let mut mask_0_bits = pcmsk0.read().bits() as u8;
        mask_0_bits |= 0b1_u8 << 4; // Button: PCINT4
        mask_0_bits |= 0b1_u8 << 5; // A: PCINT5
        pcmsk0.write(|w| w.bits(mask_0_bits));
    }

    /// Safety note: The caller must ensure that the button pin is
    /// pin change interrupt 3 of mask 0!
    pub unsafe fn snooze_button_init(
        pcicr: &Reg<PCICR_SPEC>,
        pcmsk0: &Reg<PCMSK0_SPEC>,
        _button: &pins::snooze::Button,
    ) {
        // Enable mask 0 interrupt
        let mut enabled_interrupts = pcicr.read().bits() as u8;
        enabled_interrupts |= 0b1 << 0;
        pcicr.write(|w| unsafe { w.bits(enabled_interrupts) });

        // Configure mask 0
        let mut mask_0_bits = pcmsk0.read().bits() as u8;
        mask_0_bits |= 0b1_u8 << 3; // Button: PCINT3
        pcmsk0.write(|w| w.bits(mask_0_bits));
    }

    #[avr_device::interrupt(atmega328p)]
    #[allow(non_snake_case)]
    fn PCINT2() {
        let peripherals = unsafe { Peripherals::steal() };
        let pins = pins!(peripherals);
        CHANGED_STATE.store(true, SeqCst);
        ROTARY_PIN_A.store(
            { pins.d13.into_pull_up_input() as pins::rotary_encoder::A }.is_high(),
            SeqCst,
        );
        ROTARY_PIN_B.store(
            { pins.a0.into_pull_up_input() as pins::rotary_encoder::B }.is_high(),
            SeqCst,
        );
        ROTARY_BUTTON.store(
            { pins.d12.into_pull_up_input() as pins::rotary_encoder::Button }.is_low(), // tied to gnd
            SeqCst,
        );
        SNOOZE_BUTTON.store(
            { pins.d11.into_pull_up_input() as pins::snooze::Button }.is_low(), // tied to gnd
            SeqCst,
        );
    }

    pub struct RotaryEncoderState {
        pub a: PinState,
        pub b: PinState,
        pub button: PinState,
    }

    pub fn get_rotary_encoder_state<'cs>(
        _critical_section: &CriticalSection<'cs>,
    ) -> RotaryEncoderState {
        RotaryEncoderState {
            a: ROTARY_PIN_A.load(SeqCst),
            b: ROTARY_PIN_B.load(SeqCst),
            button: ROTARY_BUTTON.load(SeqCst),
        }
    }

    pub fn get_snooze_button_pressed<'cs>(_critical_section: &CriticalSection<'cs>) -> PinState {
        SNOOZE_BUTTON.load(SeqCst)
    }

    pub fn changed_state<'cs>(_critical_section: &CriticalSection<'cs>) -> bool {
        // No compare and exchanges :(
        if CHANGED_STATE.load(SeqCst) {
            CHANGED_STATE.store(false, SeqCst);
            true
        } else {
            false
        }
    }
}

//! Pin types to improve type safety

use arduino_hal::{
    hal::port,
    port::{
        mode::{Input, Output, PullUp},
        Pin,
    },
};
use embedded_hal::digital::v2::{InputPin, OutputPin};

pub mod hours_minutes_display {
    use super::*;
    pub type SerialIn = Pin<Output, port::PD2>;
    pub type Clock = Pin<Output, port::PD3>;
    pub type Latch = Pin<Output, port::PD4>;
}

pub mod seconds_display {
    use super::*;
    pub type SerialIn = Pin<Output, port::PD5>;
    pub type Clock = Pin<Output, port::PD6>;
    pub type Latch = Pin<Output, port::PD7>;
}

pub mod character_lcd {
    use super::*;
    pub type SerialIn = Pin<Output, port::PB0>;
    pub type Clock = Pin<Output, port::PB1>;
    pub type Latch = Pin<Output, port::PB2>;
}

pub mod rotary_encoder {
    use super::*;
    pub type A = Pin<Input<PullUp>, port::PB5>;
    pub type B = Pin<Input<PullUp>, port::PC0>;
    pub type Button = Pin<Input<PullUp>, port::PB4>;
}

pub mod snooze {
    use super::*;
    pub type Button = Pin<Input<PullUp>, port::PB3>;
}

pub mod buzzer {
    use super::*;
    pub type Buzzer = Pin<Output, port::PC5>;
}

pub mod leds {
    use super::*;
    pub type Alarm = Pin<Output, port::PC1>;
    pub type PM = Pin<Output, port::PC2>;
}

pub struct ShiftRegisterPins<SerialInput, Clock, Latch>
where
    SerialInput: OutputPin,
    Clock: OutputPin,
    Latch: OutputPin,
{
    pub serial_input: SerialInput,
    pub clock: Clock,
    pub latch: Latch,
}

pub struct RotaryEncoderPins {
    pub a: rotary_encoder::A,
    pub b: rotary_encoder::B,
    pub button: rotary_encoder::Button,
}

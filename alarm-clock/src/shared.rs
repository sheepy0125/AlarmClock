use arduino_hal::{
    hal::port::{PD0, PD1},
    pac::USART0,
    port::{
        mode::{Input, Output},
        Pin,
    },
    Usart,
};

pub const BAUD_RATE: u32 = 57_600;
pub type UsbSerial = Usart<USART0, Pin<Input, PD0>, Pin<Output, PD1>>;

pub mod PinState {
    pub type PinState = bool;
    pub const LOW: bool = false;
    pub const HIGH: bool = true;
}

pub struct TimeDigits {
    pub hours: (u8, u8),
    pub minutes: (u8, u8),
    pub seconds: (u8, u8),
}
impl Default for TimeDigits {
    fn default() -> Self {
        Self {
            hours: (0_u8, 0_u8),
            minutes: (0_u8, 0_u8),
            seconds: (0_u8, 0_u8),
        }
    }
}

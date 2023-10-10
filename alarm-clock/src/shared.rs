use arduino_hal::{
    hal::port::{PD0, PD1},
    pac::USART0,
    port::{
        mode::{Input, Output},
        Pin,
    },
    Usart,
};

pub const DEBUG: bool = false;
pub const TRACE: bool = false;
pub const BAUD_RATE: u32 = 57_600_u32;
pub const UPDATE_DELTATIME: u16 = 100_u16;
/// At the expense of waiting a bit longer at start time, we can ensure that
/// our clock will continue updating in case the millis counter overflows and
/// we are waiting for a `next_update_time` that will never come.
pub const MILLIS_OVERFLOW_UPDATE_MARGIN: u32 = 5_000_u32;
pub type UsbSerial = Usart<USART0, Pin<Input, PD0>, Pin<Output, PD1>>;

pub mod PinState {
    pub type PinState = bool;
    pub const LOW: bool = false;
    pub const HIGH: bool = true;
}

#[derive(Clone)]
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

/// Time, as reported from the realtime clock
pub struct Time {
    /// Ranges from [0, 23]
    pub hours: u8,
    /// Ranges from [0, 59]
    pub minutes: u8,
    /// Ranges from [0, 59]
    pub seconds: u8,
    /// Ranges from [1, 31]
    pub day: u8,
    /// The day of the week from [0, 6] where 0 is Sunday and 6 is Saturday
    pub day_of_week: u8,
    pub month: u8,
    /// The year from 20[00-99] (Y2.1K!)
    pub year: u8,
}
impl Default for Time {
    fn default() -> Self {
        Self {
            hours: 5_u8,
            minutes: 0_u8,
            seconds: 0_u8,
            day: 1_u8,
            day_of_week: 0_u8,
            month: 1_u8,
            year: 23_u8,
        }
    }
}

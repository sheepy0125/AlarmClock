//! Types for the alarm clock's state, but not the logic behind them.
//! See main.rs for the logic!

use crate::{
    pins::{self, ShiftRegisterPins},
    shared::{Time, TimeDigits},
    shift_register::ShiftRegister,
};

/// Seconds should be tared.
pub enum TimeSetState {
    Hours(u8),
    Minutes(u8),
}

/// Everything is stored the same way with the same ranges as defined in `Time`
pub enum DateSetState {
    Day(u8),
    /// We could have this calculated, though having the user enter it is an easier
    /// solution that puts the onus on them and ensures it's always right
    DayOfWeek(u8),
    Month(u8),
    Year(u8),
}

pub enum OperationalMode {
    TimeSet(TimeSetState),
    AlarmSet(TimeSetState),
    DateSet(DateSetState),
    Idle,
    Alarm,
}

pub enum Menu {
    Idle,
    TimeSet,
    AlarmSet,
    DateSet,
    Launcher,
}

pub struct State {
    pub time: Time,
    pub alarm_time: Time,
    pub digits: TimeDigits,
    pub mode: OperationalMode,
    pub menu: Menu,
    pub alarm_enabled: bool,
    /// The next time everything *aside* from the display should update
    pub next_update: u32,
}

impl State {
    pub fn new() -> Self {
        Self {
            alarm_enabled: false,
            time: Time::default(),
            alarm_time: Time::default(),
            digits: TimeDigits::default(),
            mode: OperationalMode::Idle,
            menu: Menu::Idle,
            next_update: 0_u32,
        }
    }
}

pub trait StateLogic {
    fn handle(
        hours_minute_display_shift_register_pi: &mut ShiftRegister<
            12_usize,
            pins::hours_minutes_display::SerialIn,
            pins::hours_minutes_display::Clock,
            pins::hours_minutes_display::Latch,
        >,
    );
}

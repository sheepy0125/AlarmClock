//! NXP PCF8523 RTC

use arduino_hal::I2c;
use avr_device::interrupt::CriticalSection;
use embedded_hal::prelude::{
    _embedded_hal_blocking_i2c_Read, _embedded_hal_blocking_i2c_Write,
    _embedded_hal_blocking_i2c_WriteRead,
};

use crate::{
    console::{debug, println, trace},
    shared::{Time, TimeDigits},
};

pub const ADDRESS: u8 = 0x68_u8;
pub const READ_COMMAND: u8 = 0x03_u8;
pub const WRITE_COMMAND: u8 = 0x03_u8;

fn bcd_decode(x: u8) -> u8 {
    (((x & 0b11110000) >> 4) * 10) + (x & 0b00001111)
}

fn bcd_encode(mut x: u8) -> u8 {
    assert!(x < 100);
    let mut shift = 0_u8;
    let mut res = 0_u8;
    while x > 0 {
        res |= (x % 10) << (shift << 2);
        shift += 1;
        x /= 10;
    }
    res
}

pub struct RTC {
    pub i2c: I2c,
}
impl RTC {
    pub fn new(i2c: I2c) -> Self {
        Self { i2c }
    }

    /// Read the time, returning the time alongside updating the time digits object
    pub fn read_time(&mut self, time_digits: &mut TimeDigits) -> Option<Time> {
        let mut time_buffer = [0_u8; 7];

        trace!("[TRACE] [RTC] Reading time");

        self.i2c
            .write_read(ADDRESS, &[READ_COMMAND], &mut time_buffer)
            .map_err(|e| println!("RTC error when reading time: {:?}", e))
            .ok()?;

        time_buffer[0] &= 0b01111111;

        let seconds_digits = (time_buffer[0] >> 4, time_buffer[0] & 0b00001111);
        let minutes_digits = (time_buffer[1] >> 4, time_buffer[1] & 0b00001111);
        let hours_digits = (time_buffer[2] >> 4, time_buffer[2] & 0b00001111);

        time_digits.hours = hours_digits;
        time_digits.seconds = seconds_digits;
        time_digits.minutes = minutes_digits;

        let seconds = bcd_decode(time_buffer[0]);
        let minutes = bcd_decode(time_buffer[1]);
        let hours = bcd_decode(time_buffer[2]);
        let day = bcd_decode(time_buffer[3]);
        let day_of_week = bcd_decode(time_buffer[4]);
        let month = bcd_decode(time_buffer[5]);
        let year = bcd_decode(time_buffer[6]);

        debug!(
            "[DEBUG] [RTC] Read time: {}{}:{}{}:{}{}",
            (time_digits.hours.0 + 0x30_u8) as char,
            (time_digits.hours.1 + 0x30_u8) as char,
            (time_digits.minutes.0 + 0x30_u8) as char,
            (time_digits.minutes.1 + 0x30_u8) as char,
            (time_digits.seconds.0 + 0x30_u8) as char,
            (time_digits.seconds.1 + 0x30_u8) as char,
        );

        Some(Time {
            hours,
            minutes,
            seconds,
            day,
            day_of_week,
            month,
            year,
        })
    }

    pub fn set_time<'cs>(&mut self, time: &Time, _critical_section: &'cs CriticalSection) {
        debug!("[DEBUG] [RTC] Setting time");

        let _ = self
            .i2c
            .write(
                ADDRESS,
                &[
                    WRITE_COMMAND,
                    bcd_encode(time.seconds),
                    bcd_encode(time.minutes),
                    bcd_encode(time.hours),
                    bcd_encode(time.day),
                    bcd_encode(time.day_of_week),
                    bcd_encode(time.month),
                    bcd_encode(time.year),
                ],
            )
            .map_err(|e| println!("RTC error when setting time: {:?}", e));
    }
}

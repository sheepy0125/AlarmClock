# Alarm Clock

An Alarm Clock created as a school project.

## Hardware (general)

Uses an Arduino Uno (`atmega328p`) as the processor to control:
- 6x 7-segment digits for hours, minutes, and seconds
- 1x character LCD
- 1x snooze button (key switch)
- 1x rotary encoder to adjust time and change other settings (as seen on the LCD)
- 1x piezo buzzer
- 1x real time clock

### Shift registers

Because the Uno only has ~19 available pins, I used serial-in parallel-out buffered shift registers to send one bit at a time and release 8 bits all at once. Each shift register typically needs 2 pins at minimum (one for serial in and one for clock), however, for the seven segment displays I cut that down to 1 per and 1 total for an amortized 7 pins in total where all shift registers share one clock pulse.

## Software

All software is written in Rust with [Rahix's Arduino HAL crate](https://github.com/rahix/avr-hal).

## Design

TODO!!!

## Progress

- [x] Design schematic (done: 2023-09-17)
- [x] Order parts
- [ ] Breadboard / POC (in progress)
- [ ] Design PCB
- [ ] Order PCB parts
- [ ] Finish software (in progress)
- [x] Construct case
- [ ] Integration

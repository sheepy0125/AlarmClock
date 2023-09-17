# Alarm Clock

An Alarm Clock created as a school project in just under two weeks.

## Hardware

Uses a socketed Arduino Uno (`atmega328p`) as the processor to control:
- 6x 7-segment displays for hours, minutes, and seconds
- 1x character LCD
- 1x snooze button
- 1x rotary encoder to adjust time and change other settings (as seen on the LCD)
- 1x alarm buzzer
- 1x alarm slider switch / toggle

### Shift registers

Because the Uno only has ~19 available pins, I used serial-in parallel-out buffered shift registers to send one bit at a time and release 8 bits all at once. Each shift register typically needs 2 pins at minimum (one for serial in and one for clock), however, for the seven segment displays I cut that down to 1 per and 1 total for an amortized 7 pins in total where all shift registers share one clock pulse.

## Software

All software is written in Rust. I'm not sure why -- considering the time crunch -- but so be it.

## Design

TODO!!! Probably when the write up comes lol.

## Progress

- [x] Design schematic (done: 2023-09-17)
- [ ] Design PCB
- [ ] Order PCB parts
- [ ] Code the software
- [ ] Construct case
- [ ] Integration
- [ ] Write-up (due: 2023-09-29)

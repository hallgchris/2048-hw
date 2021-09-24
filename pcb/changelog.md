# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),

## [Unreleased]

## Version 2.0

### Added

- Add AT24C08D EEPROM memory for storing game state, etc.
- Charging IC is bypassed when the board is running and simultaneously

### Changed

- Move button and joystick GPIO pins to allow for I2C usage for EEPROM.
- Use a 10k resistor on the battery voltage divider - we now have more 10ks on the PCB.
- Changed mounting holes to M3 from M2. The top two have been lowered.
- Rotated joystick 180 degrees.
- General layout tidying.

### Removed

- Remove crystal, as it is unnecessary.

## Version 1.0

Initial PCB.

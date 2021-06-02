# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),

## [Unreleased]

### Added

- Add AT24C08D EEPROM memory for storing game state, etc.

### Changed

- Move button and joystick GPIO pins to allow for I2C usage for EEPROM.
- Use a 10k resistor on the battery voltage divider - we now have more 10ks on the PCB.

### Removed

- Remove crystal, as it is unnessary.

## Version 1.0

Initial PCB.
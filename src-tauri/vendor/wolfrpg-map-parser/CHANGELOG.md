# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0] - 2025-11-01
### Added
- Common event run condition fields parsing

### Removed
- `CommonEvent::event_type` field, replaced by `RunCondition::condition_type` field


## [0.5.5] - 2025-10-19
### Fixed
- parsing of string arguments failing for certain common events

## [0.5.4] - 2025-08-11
### Changed
- Allow invalid characters in shift-jis strings

## [0.5.3] - 2025-08-07
### Added
- `Calculation::Nothing` which is always present when assignment is `Assignment::Angle`

## [0.5.2] - 2025-08-06
### Changed
- `Calculation::BitwiseNot` is now `Calculation::Random` to better represent the engine

## [0.5.1] - 2025-08-06
### Added
- Parsing of VarDB variant of `SetVariableCommand`

### Fixed
- `SetVariableCommand` parsing gibberish instead of the correct state struct

## [0.5.0] - 2025-08-05
### Added
- Common events database parser via `db_parser::common_events_parser`

### Changed
- Separated `picture_command::display_type` in `picture_command::display_type` and `picture_command::display_operation`

### Fixed
 - DBManagement parsing failing for rare 0-string command variant
 - Command parsing failing due to certain missing signatures
 - Certain kinds of erase commands not being parsed due to picture display type not being granular enough
 - Nested loops not being parsed due to loop end signature not being checked correctly

## [0.4.2] - 2025-02-21

### Added

- Base version of `db_parser::game_data_parser`

### Changed

- Separated `db_parser` files in submodules `parsers` and `models`

### Fixed

- Small typos in documentation

## [0.4.1] - 2025-01-20

### Added

- `Clone` derive for all classes

## [0.4.0] - 2025-01-19

### Added

- New module to parse WolfRPG Editor databases and tilesets, `crate::db_parser`
- Documentation for `crate::db_parser`

### Changed

- `U32OrString` has been moved to `crate::common::u32_or_string::U32OrString`

### Fixed

- Case end not being detected properly on different nesting levels

## [0.3.3] - 2025-01-17

### Fixed

- Actual fix for Certain signatures for `CommonEventCommand` not being recognized

## [0.3.2 (YANKED)] - 2025-01-17

### Fixed

- Certain signatures for `CommonEventCommand` not being recognized
- `Page::icon_row` not being read correctly

## [0.3.1] - 2025-01-10

### Added

- `PartialEq` derive on all structs and enums

## [0.3.0] - 2025-01-10

### Added

- `serde::Deserialize` derive on all structs and enums

### Removed

- public `parse` associated functions from all structs except `map`, `event`, `page` and `command`

## [0.2.3] - 2025-01-09

### Added

- This changelog
- Documentation comments for `lib`, `map`, `event`, `page` and `command`

### Fixed
- `README.md` example relying on `args`

## [0.2.2] - 2025-01-09

### Added

- Re-exported `wolfrpg-map-parser::map::Map` as `wolfrpg-map-parser::Map`
- Usage section in `README.md`

### Changed
- Use new export in `main.rs`

## [0.2.1] - 2025-01-08

### Added

- `README.md` draft

### Fixed
- `extra` folder and other files being published on Cargo 

## [0.2.0] - 2025-01-07

### Added

- `LICENSE.md`
- All of the code
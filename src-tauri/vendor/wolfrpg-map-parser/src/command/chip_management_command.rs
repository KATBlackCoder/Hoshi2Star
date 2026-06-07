#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::chip_management_command::map_chip_settings::MapChipSettings;
use crate::command::chip_management_command::overwrite_map_chips::OverwriteMapChips;
use crate::command::chip_management_command::switch_chipset::SwitchChipset;

pub mod map_chip_settings;
pub mod options;
pub mod switch_chipset;
pub mod overwrite_map_chips;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum ChipManagementCommand {
    MapChipSettings(MapChipSettings),
    SwitchChipset(SwitchChipset),
    OverwriteMapChips(OverwriteMapChips),
}

impl ChipManagementCommand {
    pub(crate) fn parse_map_chip_settings(bytes: &[u8]) -> (usize, Self) {
        let (bytes_read, command): (usize, MapChipSettings) = MapChipSettings::parse(bytes);

        (bytes_read, Self::MapChipSettings(command))
    }

    pub(crate) fn parse_switch_chipset(bytes: &[u8]) -> (usize, Self) {
        let (bytes_read, command): (usize, SwitchChipset) = SwitchChipset::parse(bytes);

        (bytes_read, Self::SwitchChipset(command))
    }

    pub(crate) fn parse_overwrite_map_chips(bytes: &[u8]) -> (usize, Self) {
        let (bytes_read, command): (usize, OverwriteMapChips) = OverwriteMapChips::parse(bytes);

        (bytes_read, Self::OverwriteMapChips(command))
    }
}
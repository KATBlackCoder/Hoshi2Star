#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::effect_command::base::Base;
use crate::command::effect_command::change_color::ChangeColor;
use crate::command::effect_command::map_shake::MapShake;
use crate::command::effect_command::scroll_screen::ScrollScreen;

pub mod base;
pub mod map_shake;
pub mod scroll_screen;
pub mod change_color;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum EffectCommand {
    Base(Base),
    MapShake(MapShake),
    ScrollScreen(ScrollScreen),
    ChangeColor(ChangeColor)
}

impl EffectCommand {
    pub(crate) fn parse_base(bytes: &[u8]) -> (usize, Self) {
        let (bytes_read, command): (usize, Base) = Base::parse(bytes);

        (bytes_read, Self::Base(command))
    }

    pub(crate) fn parse_map_shake(bytes: &[u8]) -> (usize, Self) {
        let (bytes_read, command): (usize, MapShake) = MapShake::parse(bytes);

        (bytes_read, Self::MapShake(command))
    }

    pub(crate) fn parse_scroll_screen(bytes: &[u8]) -> (usize, Self) {
        let (bytes_read, command): (usize, ScrollScreen) = ScrollScreen::parse(bytes);

        (bytes_read, Self::ScrollScreen(command))
    }

    pub(crate) fn parse_change_color(bytes: &[u8]) -> (usize, Self) {
        let (bytes_read, command): (usize, ChangeColor) = ChangeColor::parse(bytes);

        (bytes_read, Self::ChangeColor(command))
    }
}
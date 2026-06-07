#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::effect_command::base::character_effect_type::CharacterEffectType;
use crate::command::effect_command::base::map_effect_type::MapEffectType;
use crate::command::effect_command::base::picture_effect_type::PictureEffectType;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum EffectType {
    Picture(PictureEffectType),
    Character(CharacterEffectType),
    Map(MapEffectType),
    Unknown
}
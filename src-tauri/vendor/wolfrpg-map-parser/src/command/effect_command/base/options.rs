#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::effect_command::base::character_effect_type::CharacterEffectType;
use crate::command::effect_command::base::effect_target::EffectTarget;
use crate::command::effect_command::base::effect_type::EffectType;
use crate::command::effect_command::base::map_effect_type::MapEffectType;
use crate::command::effect_command::base::picture_effect_type::PictureEffectType;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Options {
    target: EffectTarget,
    effect_type: EffectType
}

impl Options {
    pub fn new(options: u32) -> Self {
        let target: EffectTarget = EffectTarget::new((options & 0x0f) as u8);
        let effect_type: u8 = ((options & 0xf0) >> 4) as u8;

        let effect_type: EffectType = match target {
            EffectTarget::Picture => EffectType::Picture(PictureEffectType::new(effect_type)),
            EffectTarget::Character => EffectType::Character(CharacterEffectType::new(effect_type)),
            EffectTarget::Map => EffectType::Map(MapEffectType::new(effect_type)),
            EffectTarget::Unknown => EffectType::Unknown
        };

        Self {
            target,
            effect_type
        }
    }

    pub fn target(&self) -> &EffectTarget {
        &self.target
    }
    
    pub fn target_mut(&mut self) -> &mut EffectTarget {
        &mut self.target
    }

    pub fn effect_type(&self) -> &EffectType {
        &self.effect_type
    }
    
    pub fn effect_type_mut(&mut self) -> &mut EffectType {
        &mut self.effect_type
    }
}
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum SpecialOperation {
    PushCharactersToFront   = 0x00,
    EraseAllCharacters      = 0x01,
    WarpPartyToHero         = 0x02,
    StartHeroPartySynchro   = 0x03,
    CancelHeroPartySynchro  = 0x04,
    MakePartyTransparent    = 0x05,
    CancelPartyTransparency = 0x06,
    SavePartyMembers        = 0x07,
    LoadPartyMembers        = 0x08,
    TurnOnPartyFollowing    = 0x09,
    TurnOffPartyFollowing   = 0x0a,
    Unknown
}

impl SpecialOperation {
    pub const fn new(operation: u8) -> Self {
        match operation {
            0x00 => SpecialOperation::PushCharactersToFront,
            0x01 => SpecialOperation::EraseAllCharacters,
            0x02 => SpecialOperation::WarpPartyToHero,
            0x03 => SpecialOperation::StartHeroPartySynchro,
            0x04 => SpecialOperation::CancelHeroPartySynchro,
            0x05 => SpecialOperation::MakePartyTransparent,
            0x06 => SpecialOperation::CancelPartyTransparency,
            0x07 => SpecialOperation::SavePartyMembers,
            0x08 => SpecialOperation::LoadPartyMembers,
            0x09 => SpecialOperation::TurnOnPartyFollowing,
            0x0a => SpecialOperation::TurnOffPartyFollowing,
            _ => SpecialOperation::Unknown
        }
    }
}
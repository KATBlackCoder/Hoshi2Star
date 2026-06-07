#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum MoveType {
    MoveDown            = 0x0000,
    MoveLeft            = 0x0001,
    MoveRight           = 0x0002,
    MoveUp              = 0x0003,
    MoveDownLeft        = 0x0004,
    MoveDownRight       = 0x0005,
    MoveUpLeft          = 0x0006,
    MoveUpRight         = 0x0007,

    FaceDown            = 0x0008,
    FaceLeft            = 0x0009,
    FaceRight           = 0x000a,
    FaceUp              = 0x000b,
    FaceDownLeft        = 0x000c,
    FaceDownRight       = 0x000d,
    FaceUpLeft          = 0x000e,
    FaceUpRight         = 0x000f,

    MoveRandom          = 0x0010,
    MoveTowardHero      = 0x0011,
    MoveAwayFromHero    = 0x0012,
    StepForward         = 0x0013,
    StepBackward        = 0x0014,

    TurnRight           = 0x0016,
    TurnLeft            = 0x0017,
    TurnLeftRightRandom = 0x0018,
    FaceRandomDirection = 0x0019,
    FaceTowardHero      = 0x001a,
    FaceAwayFromHero    = 0x001b,

    IdleAnimationOn     = 0x0020,
    IdleAnimationOff    = 0x0021,
    MoveAnimationOn     = 0x0022,
    MoveAnimationOff    = 0x0023,
    FixedDirectionOn    = 0x0024,
    FixedDirectionOff   = 0x0025,
    SlipThroughOn       = 0x0026,
    SlipThroughOff      = 0x0027,
    AlwaysOnTopOn       = 0x0028,
    AlwaysOnTopOff      = 0x0029,

    SetHalfTileMovement = 0x0030,
    SetFullTileMovement = 0x0031,

    SwitchToPattern1    = 0x0032,
    SwitchToPattern2    = 0x0033,
    SwitchToPattern3    = 0x0034,
    SwitchToPattern4    = 0x0038,
    SwitchToPattern5    = 0x0039,

    SetMoveSpeed        = 0x011d,
    SetMoveFrequency    = 0x011e,
    SetAnimationSpeed   = 0x011f,

    SetGraphic          = 0x012c,
    SetOpacity          = 0x012d,
    PlaySound           = 0x012e,
    WaitFrames          = 0x012f,

    ApproachEvent       = 0x0135,
    SetHeight           = 0x013a,

    Jump                = 0x0215,
    SetVariable         = 0x021c,

    ApproachPosition    = 0x0236,

    AddToVariable       = 0x0237,
    Unknown
}

impl MoveType {
    pub const fn new(move_type: u16) -> Self {
        match move_type {
            0x0000 => Self::MoveDown,
            0x0001 => Self::MoveLeft,
            0x0002 => Self::MoveRight,
            0x0003 => Self::MoveUp,
            0x0004 => Self::MoveDownLeft,
            0x0005 => Self::MoveDownRight,
            0x0006 => Self::MoveUpLeft,
            0x0007 => Self::MoveUpRight,

            0x0008 => Self::FaceDown,
            0x0009 => Self::FaceLeft,
            0x000a => Self::FaceRight,
            0x000b => Self::FaceUp,
            0x000c => Self::FaceDownLeft,
            0x000d => Self::FaceDownRight,
            0x000e => Self::FaceUpLeft,
            0x000f => Self::FaceUpRight,

            0x0010 => Self::MoveRandom,
            0x0011 => Self::MoveTowardHero,
            0x0012 => Self::MoveAwayFromHero,
            0x0013 => Self::StepForward,
            0x0014 => Self::StepBackward,

            0x0016 => Self::TurnRight,
            0x0017 => Self::TurnLeft,
            0x0018 => Self::TurnLeftRightRandom,
            0x0019 => Self::FaceRandomDirection,
            0x001a => Self::FaceTowardHero,
            0x001b => Self::FaceAwayFromHero,

            0x0020 => Self::IdleAnimationOn,
            0x0021 => Self::IdleAnimationOff,
            0x0022 => Self::MoveAnimationOn,
            0x0023 => Self::MoveAnimationOff,
            0x0024 => Self::FixedDirectionOn,
            0x0025 => Self::FixedDirectionOff,
            0x0026 => Self::SlipThroughOn,
            0x0027 => Self::SlipThroughOff,
            0x0028 => Self::AlwaysOnTopOn,
            0x0029 => Self::AlwaysOnTopOff,

            0x0030 => Self::SetHalfTileMovement,
            0x0031 => Self::SetFullTileMovement,

            0x0032 => Self::SwitchToPattern1,
            0x0033 => Self::SwitchToPattern2,
            0x0034 => Self::SwitchToPattern3,
            0x0038 => Self::SwitchToPattern4,
            0x0039 => Self::SwitchToPattern5,

            0x011d => Self::SetMoveSpeed,
            0x011e => Self::SetMoveFrequency,
            0x011f => Self::SetAnimationSpeed,

            0x012c => Self::SetGraphic,
            0x012d => Self::SetOpacity,
            0x012e => Self::PlaySound,
            0x012f => Self::WaitFrames,

            0x0135 => Self::ApproachEvent,
            0x013a => Self::SetHeight,

            0x0215 => Self::Jump,
            0x021c => Self::SetVariable,

            0x0236 => Self::ApproachPosition,

            0x0237 => Self::AddToVariable,

            _ => Self::Unknown
        }
    }
}
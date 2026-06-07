#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum StringOperation {
    Equals              = 0x00,
    PlusEquals          = 0x01,
    CopyFirstLine       = 0x02,
    CutFirstLine        = 0x03,
    CutFirstCharacter   = 0x04,
    LoadFileContents    = 0x05,
    ExportToFile        = 0x06,
    GetFileList         = 0x07,
    RemoveInstancesOf   = 0x08,
    Replace             = 0x09,
    CutUpTo             = 0x0a,
    CutAfter            = 0x0b,
    Unknown,
}

impl StringOperation {
    pub const fn new(operation: u8) -> StringOperation {
        match operation {
            0x00 => StringOperation::Equals,
            0x01 => StringOperation::PlusEquals,
            0x02 => StringOperation::CopyFirstLine,
            0x03 => StringOperation::CutFirstLine,
            0x04 => StringOperation::CutFirstCharacter,
            0x05 => StringOperation::LoadFileContents,
            0x06 => StringOperation::ExportToFile,
            0x07 => StringOperation::GetFileList,
            0x08 => StringOperation::RemoveInstancesOf,
            0x09 => StringOperation::Replace,
            0x0a => StringOperation::CutUpTo,
            0x0b => StringOperation::CutAfter,
            _ => StringOperation::Unknown,
        }
    }
}
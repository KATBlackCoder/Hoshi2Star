#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::picture_command::anchor::Anchor;
use crate::command::picture_command::blending_method::BlendingMethod;
use crate::command::picture_command::display_operation::DisplayOperation;
use crate::command::picture_command::display_type::DisplayType;
use crate::command::picture_command::zoom::Zoom;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Options {
    display_operation: DisplayOperation,
    display_type: DisplayType,
    blending_method: BlendingMethod,
    anchor: Anchor,
    position_relative: bool,
    zoom: Zoom,
    range: bool,
    link_to_scroll: bool,
    free_transform: bool,
}

impl Options {
    pub fn new(options: u32) -> Self {
        Self {
            display_operation: DisplayOperation::new((options & 0xff) as u8),
            display_type: DisplayType::new((options & 0xff) as u8),
            blending_method: BlendingMethod::new(((options >> 8) & 0x0f) as u8),
            anchor: Anchor::new((((options >> 8) & 0xf0) >> 4) as u8),
            position_relative: ((options >> 16) & 0b00000001) != 0,
            zoom: Zoom::new((((options >> 16) & 0xf0) >> 4) as u8),
            range:          (options >> 24) & 0b00000001 != 0,
            link_to_scroll: (options >> 24) & 0b00000010 != 0,
            free_transform: (options >> 24) & 0b00000100 != 0
        }
    }

    pub fn display_operation(&self) -> &DisplayOperation {
        &self.display_operation
    }

    pub fn display_type(&self) -> &DisplayType {
        &self.display_type
    }

    pub fn blending_method(&self) -> &BlendingMethod {
        &self.blending_method
    }

    pub fn anchor(&self) -> &Anchor {
        &self.anchor
    }

    pub fn position_relative(&self) -> bool {
        self.position_relative
    }

    pub fn zoom(&self) -> &Zoom {
        &self.zoom
    }

    pub fn range(&self) -> bool {
        self.range
    }

    pub fn link_to_scroll(&self) -> bool {
        self.link_to_scroll
    }

    pub fn free_transform(&self) -> bool {
        self.free_transform
    }
}
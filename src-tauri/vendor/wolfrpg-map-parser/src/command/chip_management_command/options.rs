#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Options {
    no_down: bool,
    no_left: bool,
    no_right: bool,
    no_up: bool,
    above_hero: bool,
    half_transparent: bool,
    counter: bool,
    match_lower_layer: bool,
}

impl Options {
    pub fn new(options: u32) -> Self {
        Self {
            no_down:            options & 0b00000001 != 0,
            no_left:            options & 0b00000010 != 0,
            no_right:           options & 0b00000100 != 0,
            no_up:              options & 0b00001000 != 0,
            above_hero:         options & 0b00010000 != 0,
            half_transparent:   options & 0b01000000 != 0,
            counter:            options & 0b10000000 != 0,

            match_lower_layer:  (options >> 8) & 0b00000010 != 0,
        }
    }

    pub fn no_down(&self) -> bool {
        self.no_down
    }
    
    pub fn no_down_mut(&mut self) -> &mut bool {
        &mut self.no_down
    }

    pub fn no_left(&self) -> bool {
        self.no_left
    }
    
    pub fn no_left_mut(&mut self) -> &mut bool {
        &mut self.no_left
    }

    pub fn no_right(&self) -> bool {
        self.no_right
    }
    
    pub fn no_right_mut(&mut self) -> &mut bool {
        &mut self.no_right
    }

    pub fn no_up(&self) -> bool {
        self.no_up
    }
    
    pub fn no_up_mut(&mut self) -> &mut bool {
        &mut self.no_up
    }

    pub fn above_hero(&self) -> bool {
        self.above_hero
    }
    
    pub fn above_hero_mut(&mut self) -> &mut bool {
        &mut self.above_hero
    }

    pub fn half_transparent(&self) -> bool {
        self.half_transparent
    }
    
    pub fn half_transparent_mut(&mut self) -> &mut bool {
        &mut self.half_transparent
    }

    pub fn counter(&self) -> bool {
        self.counter
    }
    
    pub fn counter_mut(&mut self) -> &mut bool {
        &mut self.counter
    }

    pub fn match_lower_layer(&self) -> bool {
        self.match_lower_layer
    }
    
    pub fn match_lower_layer_mut(&mut self) -> &mut bool {
        &mut self.match_lower_layer
    }
}
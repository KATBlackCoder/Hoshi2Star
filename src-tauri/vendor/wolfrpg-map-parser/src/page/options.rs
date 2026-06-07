#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Options {
    idle_animation: bool,
    move_animation: bool,
    fixed_direction: bool,
    slip_through: bool,
    above_hero: bool,
    square_hitbox: bool,
    half_step_up: bool,
}

impl Options {
    pub fn new(options: u8) -> Self {
        Self {
            idle_animation:     options & 0b00000001 != 0,
            move_animation:     options & 0b00000010 != 0,
            fixed_direction:    options & 0b00000100 != 0,
            slip_through:       options & 0b00001000 != 0,
            above_hero:         options & 0b00010000 != 0,
            square_hitbox:      options & 0b00100000 != 0,
            half_step_up:       options & 0b01000000 != 0,
        }
    }

    pub fn idle_animation(&self) -> bool {
        self.idle_animation
    }

    pub fn idle_animation_mut(&mut self) -> &mut bool {
        &mut self.idle_animation
    }

    pub fn move_animation(&self) -> bool {
        self.move_animation
    }

    pub fn move_animation_mut(&mut self) -> &mut bool {
        &mut self.move_animation
    }

    pub fn fixed_direction(&self) -> bool {
        self.fixed_direction
    }

    pub fn fixed_direction_mut(&mut self) -> &mut bool {
        &mut self.fixed_direction
    }

    pub fn slip_through(&self) -> bool {
        self.slip_through
    }

    pub fn slip_through_mut(&mut self) -> &mut bool {
        &mut self.slip_through
    }

    pub fn above_hero(&self) -> bool {
        self.above_hero
    }

    pub fn above_hero_mut(&mut self) -> &mut bool {
        &mut self.above_hero
    }

    pub fn square_hitbox(&self) -> bool {
        self.square_hitbox
    }

    pub fn square_hitbox_mut(&mut self) -> &mut bool {
        &mut self.square_hitbox
    }

    pub fn half_step_up(&self) -> bool {
        self.half_step_up
    }

    pub fn half_step_up_mut(&mut self) -> &mut bool {
        &mut self.half_step_up
    }
}
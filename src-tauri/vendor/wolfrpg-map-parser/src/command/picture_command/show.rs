use crate::byte_utils::{as_u32_le, parse_string};
use crate::common::u32_or_string::U32OrString;
use crate::command::picture_command::display_type::DisplayType;
use crate::command::picture_command::options::Options;
use crate::byte_utils::parse_optional_string;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use state::State;

pub mod state;
pub mod base;
pub mod free_transform;
pub mod delay;
mod parser;
pub mod zoom;
pub mod color_values;
mod range_fields;
mod color_values_fields;
mod zoom_fields;
mod free_transform_fields;
mod delay_fields;
pub mod range;
mod colors_fields;
pub mod colors;
mod parsable_fields;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Show {
    options: Options,
    picture: u32,
    process_time: u32,
    division_width: u32,
    division_height: u32,
    pattern: u32,
    opacity: u32,
    zoom: u32,
    angle: u32,
    state: State,
    filename: Option<U32OrString>,
    string: Option<String>,
}

type StateParser = fn(&[u8], &Options) -> (usize, Option<u32>, State);

impl Show {
    fn parse(bytes: &[u8], parse_state: StateParser) -> (usize, Self) {
        let mut offset: usize = 0;

        let options: u32 = as_u32_le(&bytes[offset..offset+4]);
        let options: Options = Options::new(options);
        offset += 4;

        let picture: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let process_time: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let division_width: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let division_height: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let pattern: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let opacity: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let zoom: u32 = as_u32_le(&bytes[offset+8..offset+12]);
        let angle: u32 = as_u32_le(&bytes[offset+12..offset+16]);

        let (bytes_read, filename_variable, state): (usize, Option<u32>, State)
            = parse_state(&bytes[offset..], &options);
        offset += bytes_read;

        let (bytes_read, string_value): (usize, Option<String>)
            = Self::parse_string_value(&bytes[offset..]);
        offset += bytes_read;

        let (filename, string): (Option<U32OrString>, Option<String>)
            = Self::make_filename_and_string(string_value, filename_variable, &options);

        offset += 1; // Command end signature

        (offset, Self {
            options,
            picture,
            process_time,
            division_width,
            division_height,
            pattern,
            opacity,
            zoom,
            angle,
            state,
            filename,
            string
        })
    }
    fn parse_string_value(bytes: &[u8]) -> (usize, Option<String>) {
        let mut offset: usize = 0;

        let is_filename_string: bool = bytes[offset] != 0;
        offset += 1;

        let string_value: Option<String>
            = parse_optional_string!(bytes, offset, is_filename_string);

        (offset, string_value)
    }

    fn make_filename_and_string(string_value: Option<String>, filename_variable: Option<u32>,
                                    options: &Options) -> (Option<U32OrString>, Option<String>) {
        let (filename, string): (Option<String>, Option<String>) = match *options.display_type() {
            DisplayType::StringAsPicture => (None, string_value),
            _ => (string_value, None)
        };

        let filename: Option<U32OrString> = match filename {
            Some(filename) => Some(U32OrString::String(filename)),
            None => filename_variable.map(U32OrString::U32)
        };

        (filename, string)
    }

    pub(crate) fn parse_base(bytes: &[u8]) -> (usize, Self) {
        Self::parse(bytes, State::parse_base)
    }

    pub(crate) fn parse_colors(bytes: &[u8]) -> (usize, Self) {
        Self::parse(bytes, State::parse_colors)
    }

    pub(crate) fn parse_delay(bytes: &[u8]) -> (usize, Self) {
        Self::parse(bytes, State::parse_delay)
    }

    pub(crate) fn parse_range(bytes: &[u8]) -> (usize, Self) {
        Self::parse(bytes, State::parse_range)
    }

    pub(crate) fn parse_color_values(bytes: &[u8]) -> (usize, Self) {
        Self::parse(bytes, State::parse_color_values)
    }

    pub(crate) fn parse_zoom(bytes: &[u8]) -> (usize, Self) {
        Self::parse(bytes, State::parse_zoom)
    }

    pub(crate) fn parse_free_transform(bytes: &[u8]) -> (usize, Self) {
        Self::parse(bytes, State::parse_free_transform)
    }

    pub fn options(&self) -> &Options {
        &self.options
    }

    pub fn options_mut(&mut self) -> &mut Options {
        &mut self.options
    }

    pub fn picture(&self) -> u32 {
        self.picture
    }

    pub fn picture_mut(&mut self) -> &mut u32 {
        &mut self.picture
    }

    pub fn process_time(&self) -> u32 {
        self.process_time
    }

    pub fn process_time_mut(&mut self) -> &mut u32 {
        &mut self.process_time
    }

    pub fn division_width(&self) -> u32 {
        self.division_width
    }

    pub fn division_width_mut(&mut self) -> &mut u32 {
        &mut self.division_width
    }

    pub fn division_height(&self) -> u32 {
        self.division_height
    }

    pub fn division_height_mut(&mut self) -> &mut u32 {
        &mut self.division_height
    }

    pub fn pattern(&self) -> u32 {
        self.pattern
    }

    pub fn pattern_mut(&mut self) -> &mut u32 {
        &mut self.pattern
    }

    pub fn opacity(&self) -> u32 {
        self.opacity
    }

    pub fn opacity_mut(&mut self) -> &mut u32 {
        &mut self.opacity
    }

    pub fn zoom(&self) -> u32 {
        self.zoom
    }

    pub fn zoom_mut(&mut self) -> &mut u32 {
        &mut self.zoom
    }

    pub fn angle(&self) -> u32 {
        self.angle
    }

    pub fn angle_mut(&mut self) -> &mut u32 {
        &mut self.angle
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    pub fn filename(&self) -> &Option<U32OrString> {
        &self.filename
    }

    pub fn filename_mut(&mut self) -> &mut Option<U32OrString> {
        &mut self.filename
    }

    pub fn string(&self) -> &Option<String> {
        &self.string
    }

    pub fn string_mut(&mut self) -> &mut Option<String> {
        &mut self.string
    }
}
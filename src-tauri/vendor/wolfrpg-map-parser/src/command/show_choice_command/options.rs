#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::show_choice_command::cancel_case::CancelCase;
use crate::command::show_choice_command::extra_cases::ExtraCases;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Options {
    cancel_case: CancelCase,
    selected_choices: u8,
    extra_cases: ExtraCases
}

impl Options {
    pub fn new(options: u16) -> Self {
        let selected_choices: u8 = (options & 0xff) as u8;
        let extra_cases: u8 = ((options >> 8) & 0xff) as u8;
        Self {
            cancel_case: CancelCase::new((selected_choices >> 4) & 0b00001111),
            selected_choices: selected_choices & 0b00001111,
            extra_cases: ExtraCases::new(extra_cases)
        }
    }

    pub fn case_count(&self) -> usize {
        self.selected_choices as usize
        + self.extra_cases.count()
        + match self.cancel_case {
            CancelCase::Separate => 1,
            _ => 0
        }
    }

    pub fn cancel_case(&self) -> &CancelCase {
        &self.cancel_case
    }

    pub fn cancel_case_mut(&mut self) -> &mut CancelCase {
        &mut self.cancel_case
    }

    pub fn selected_choices(&self) -> u8 {
        self.selected_choices
    }

    pub fn selected_choices_mut(&mut self) -> &mut u8 {
        &mut self.selected_choices
    }

    pub fn extra_cases(&self) -> &ExtraCases {
        &self.extra_cases
    }

    pub fn extra_cases_mut(&mut self) -> &mut ExtraCases {
        &mut self.extra_cases
    }
}
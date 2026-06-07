#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::event_control_command::erase_event::EraseEvent;
use crate::command::event_control_command::label::Label;
use crate::command::event_control_command::loop_command::Loop;
use crate::command::event_control_command::loop_count::LoopCount;
use crate::command::event_control_command::move_route::MoveRoute;
use crate::command::event_control_command::set_transition::SetTransition;
use crate::command::event_control_command::wait::Wait;

pub mod loop_command;
pub mod set_transition;
pub mod move_route;
pub mod erase_event;
pub mod wait;
pub mod loop_count;
pub mod label;

const COMMAND_END_SIGNATURE_LENGTH: usize = 3;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum EventControlCommand {
    Loop(Loop),
    BreakLoop,
    GotoLoopStart,
    PrepareTransition,
    ExecuteTransition,
    SetTransition(SetTransition),
    MoveRoute(MoveRoute),
    WaitForMoveRoute,
    MoveDuringEventsOn,
    MoveDuringEventsOff,
    GotoTitle,
    GameEnd,
    StopNonPictureGraphicUpdates,
    ResumeNonPictureGraphicUpdates,
    ForceExitEvent,
    EraseEvent(EraseEvent),
    Wait(Wait),
    LoopCount(LoopCount),
    LabelPoint(Label),
    LabelJump(Label)
}

impl EventControlCommand {
    fn parse_empty_command(command: EventControlCommand) -> (usize, Self) {
        (COMMAND_END_SIGNATURE_LENGTH, command)
    }
    pub(crate) fn parse_loop(bytes: &[u8]) -> (usize, u32, Self) {
        let (bytes_read, commands_read, command): (usize, u32, Loop) = Loop::parse(bytes);

        (bytes_read, commands_read, Self::Loop(command))
    }

    pub(crate) fn parse_break_loop(_: &[u8]) -> (usize, Self) {
        Self::parse_empty_command(Self::BreakLoop)
    }

    pub(crate) fn parse_goto_loop_start(_: &[u8]) -> (usize, Self) {
        Self::parse_empty_command(Self::GotoLoopStart)
    }

    pub(crate) fn parse_prepare_transition(_: &[u8]) -> (usize, Self) {
        Self::parse_empty_command(Self::PrepareTransition)
    }

    pub(crate) fn parse_execute_transition(_: &[u8]) -> (usize, Self) {
        Self::parse_empty_command(Self::ExecuteTransition)
    }

    pub(crate) fn parse_set_transition(bytes: &[u8]) -> (usize, Self) {
        let (bytes_read, command): (usize, SetTransition) = SetTransition::parse(bytes);

        (bytes_read, Self::SetTransition(command))
    }

    pub(crate) fn parse_move_route(bytes: &[u8]) -> (usize, Self) {
        let (bytes_read, command): (usize, MoveRoute) = MoveRoute::parse(bytes);

        (bytes_read, Self::MoveRoute(command))
    }

    pub(crate) fn parse_wait_for_move_route(_: &[u8]) -> (usize, Self) {
        Self::parse_empty_command(Self::WaitForMoveRoute)
    }

    pub(crate) fn parse_move_during_events_on(_: &[u8]) -> (usize, Self) {
        Self::parse_empty_command(Self::MoveDuringEventsOn)
    }

    pub(crate) fn parse_move_during_events_off(_: &[u8]) -> (usize, Self) {
        Self::parse_empty_command(Self::MoveDuringEventsOff)
    }

    pub(crate) fn parse_goto_title(_: &[u8]) -> (usize, Self) {
        Self::parse_empty_command(Self::GotoTitle)
    }

    pub(crate) fn parse_game_end(_: &[u8]) -> (usize, Self) {
        Self::parse_empty_command(Self::GameEnd)
    }

    pub(crate) fn parse_stop_non_picture_graphic_updates(_: &[u8]) -> (usize, Self) {
        Self::parse_empty_command(Self::StopNonPictureGraphicUpdates)
    }

    pub(crate) fn parse_resume_non_picture_graphic_updates(_: &[u8]) -> (usize, Self) {
        Self::parse_empty_command(Self::ResumeNonPictureGraphicUpdates)
    }

    pub(crate) fn parse_force_exit_event(_: &[u8]) -> (usize, Self) {
        Self::parse_empty_command(Self::ForceExitEvent)
    }

    pub(crate) fn parse_erase_event(bytes: &[u8]) -> (usize, Self) {
        let (bytes_read, command): (usize, EraseEvent) = EraseEvent::parse(bytes);

        (bytes_read, Self::EraseEvent(command))
    }

    pub(crate) fn parse_wait(bytes: &[u8]) -> (usize, Self) {
        let (bytes_read, command): (usize, Wait) = Wait::parse(bytes);

        (bytes_read, Self::Wait(command))
    }

    pub(crate) fn parse_loop_count(bytes: &[u8]) -> (usize, u32, Self) {
        let (bytes_read, commands_read, command): (usize, u32, LoopCount) = LoopCount::parse(bytes);

        (bytes_read, commands_read, Self::LoopCount(command))
    }

    pub(crate) fn parse_label_point(bytes: &[u8]) -> (usize, Self) {
        let (bytes_read, command): (usize, Label) = Label::parse(bytes);

        (bytes_read, Self::LabelPoint(command))
    }

    pub(crate) fn parse_label_jump(bytes: &[u8]) -> (usize, Self) {
        let (bytes_read, command): (usize, Label) = Label::parse(bytes);

        (bytes_read, Self::LabelJump(command))
    }
 }
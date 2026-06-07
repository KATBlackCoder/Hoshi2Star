#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::as_u32_be;
use show_choice_command::ShowChoiceCommand;
use show_text_command::ShowTextCommand;
use crate::command::chip_management_command::ChipManagementCommand;
use crate::command::common_event_command::CommonEventCommand;
use crate::command::db_management_command::DBManagementCommand;
use crate::command::effect_command::EffectCommand;
use crate::command::event_control_command::EventControlCommand;
use crate::command::input_key_command::InputKeyCommand;
use crate::command::number_condition_command::NumberConditionCommand;
use crate::command::party_graphics_command::PartyGraphicsCommand;
use crate::command::picture_command::PictureCommand;
use crate::command::save_load_command::SaveLoadCommand;
use crate::command::set_string_command::SetStringCommand;
use crate::command::set_variable_command::SetVariableCommand;
use crate::command::set_variable_plus_command::SetVariablePlusCommand;
use crate::command::signature::Signature;
use crate::command::sound_command::SoundCommand;
use crate::command::string_condition_command::StringConditionCommand;
use crate::command::transfer_command::TransferCommand;

pub mod show_choice_command;
pub mod show_text_command;
pub mod set_variable_command;
pub mod db_management_command;
pub mod common;
pub mod set_string_command;
pub mod set_variable_plus_command;
pub mod number_condition_command;
pub mod string_condition_command;
pub mod input_key_command;
pub mod picture_command;
pub mod effect_command;
pub mod sound_command;
pub mod save_load_command;
pub mod party_graphics_command;
pub mod chip_management_command;
pub mod transfer_command;
pub mod event_control_command;
pub mod common_event_command;
mod signature;

/// An event command instruction.
/// 
/// A command can be anything from a single instruction to a loop containing other instructions.
/// Because of the inherently-recursive nature of this enum, it is probably best to create new
/// traits and implement them for each variant.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum Command {
    ShowMessage(ShowTextCommand),
    Comment(ShowTextCommand),
    DebugText(ShowTextCommand),
    ForceCloseMessage(),
    ClearDebugText(),
    ShowChoice(ShowChoiceCommand),
    SetVariable(SetVariableCommand),
    DBManagement(DBManagementCommand),
    SetString(SetStringCommand),
    SetVariablePlus(SetVariablePlusCommand),
    NumberCondition(NumberConditionCommand),
    StringCondition(StringConditionCommand),
    InputKey(InputKeyCommand),
    Picture(PictureCommand),
    Effect(EffectCommand),
    Sound(SoundCommand),
    SaveLoad(SaveLoadCommand),
    PartyGraphics(PartyGraphicsCommand),
    ChipManagement(ChipManagementCommand),
    Transfer(TransferCommand),
    EventControl(EventControlCommand),
    CommonEvent(CommonEventCommand),
    Checkpoint(),
    Exit(),
}

impl Command {
    /// Parse raw bytes into a single [`Command`] struct.
    ///
    /// Use of this method is highly discouraged unless you know exactly what you are doing.
    /// Prefer using [`Map::parse`] and then extract what you want from the structure tree.
    ///
    /// # Panics
    /// This function will panic if the given bytes do not represent a valid command structure.
    ///
    /// This might be caused by unaligned bytes, corrupt files, incompatible format updates and
    /// library bugs.
    /// If you are confident you are doing everything right, feel free to report an issue on [GitHub].
    ///
    /// [`Map::parse`]: crate::map::Map::parse
    /// [GitHub]: https://github.com/G1org1owo/wolfrpg-map-parser/issues
    pub fn parse(bytes: &[u8]) -> (usize, u32, Self) {
        let mut offset: usize = 0;
        let commands: u32 = 1;

        let signature: u32 = as_u32_be(&bytes[offset..offset+4]);
        offset += 4;
        offset += 1; // padding

        let command_parser: fn(&[u8], u32) -> (usize, u32, Self) = match Signature::new(signature) {
            Signature::ShowMessage => Self::parse_show_message,
            Signature::Comment => Self::parse_comment,
            Signature::DebugText => Self::parse_debug_text,
            Signature::ForceCloseMessage => Self::parse_force_close_message,
            Signature::ClearDebugText => Self::parse_clear_debug_text,
            Signature::ShowChoice => Self::parse_show_choice,
            Signature::SetVariableBase => Self::parse_set_variable_base,
            Signature::SetVariableRange => Self::parse_set_variable_range,
            Signature::SetVariableDB => Self::parse_set_variable_db,
            Signature::DBManagementBase => Self::parse_db_management_base,
            Signature::DBManagementString => Self::parse_db_management_string,
            Signature::DBManagementCsv => Self::parse_db_management_csv,
            Signature::SetStringBase => Self::parse_set_string_base,
            Signature::SetStringDynamic => Self::parse_set_string_dynamic,
            Signature::SetVariablePlusBase => Self::parse_set_variable_plus_base,
            Signature::SetVariablePlusOther => Self::parse_set_variable_plus_other,

            Signature::NumberCondition | Signature::NumberConditionDouble |
            Signature::NumberConditionTriple => Self::parse_number_condition,

            Signature::StringCondition | Signature::StringConditionTwo |
            Signature::StringConditionThree | Signature::StringConditionFour |
            Signature::StringConditionFive | Signature::StringConditionSix |
            Signature::StringConditionSeven | Signature::StringConditionEight
                => Self::parse_string_condition,

            Signature::InputKeyBase => Self::parse_input_key_base,
            Signature::InputKeyKeyboardOrPad => Self::parse_input_key_keyboard_or_pad,

            Signature::AutomaticInputBasic | Signature::AutomaticInputMouse
                => Self::parse_automatic_input_base,

            Signature::AutomaticInputKeyboard => Self::parse_automatic_input_keyboard,

            Signature::InputToggleBasic | Signature::InputToggleDevice
                => Self::parse_input_toggle,

            Signature::PictureShowBase | Signature::PictureShowBaseByVar
                => Self::parse_picture_show_base,

            Signature::PictureShowColors => Self::parse_picture_show_colors,
            Signature::PictureShowDelay => Self::parse_picture_show_delay,
            Signature::PictureShowRange => Self::parse_picture_show_range,
            Signature::PictureShowColorValues => Self::parse_picture_show_color_values,
            Signature::PictureShowZoom => Self::parse_picture_show_zoom,
            Signature::PictureShowFreeTransform => Self::parse_picture_show_free_transform,
            Signature::PictureEraseDelayReset => Self::parse_picture_erase_delay_reset,
            Signature::PictureEraseBase => Self::parse_picture_erase_base,
            Signature::PictureEraseDelay => Self::parse_picture_erase_delay,
            Signature::PictureEraseRange => Self::parse_picture_erase_range,
            Signature::EffectBase => Self::parse_effect_base,
            Signature::EffectMapShake => Self::parse_effect_map_shake,
            Signature::EffectScrollScreen => Self::parse_effect_scroll_screen,
            Signature::EffectChangeColor => Self::parse_effect_change_color,

            Signature::SoundFilename | Signature::SoundFilenameSe => Self::parse_sound_filename,

            Signature::SoundVariable => Self::parse_sound_variable,

            Signature::SoundFreeAll | Signature::SoundFreeAllVariable => Self::parse_sound_free_all,

            Signature::SaveLoadBase => Self::parse_save_load_base,
            Signature::LoadVariable => Self::parse_load_variable,
            Signature::SaveVariable => Self::parse_save_variable,

            Signature::PartyGraphicsBase | Signature::PartyGraphicsVariable |
            Signature::PartyGraphicsNoMember => Self::parse_party_graphics,

            Signature::ChipManagementSettings => Self::parse_chip_management_settings,
            Signature::ChipManagementSwitchSet => Self::parse_chip_management_switch_set,
            Signature::ChipManagementOverwrite => Self::parse_chip_management_overwrite,
            Signature::Transfer => Self::parse_transfer,
            Signature::Loop => Self::parse_loop,
            Signature::BreakLoop => Self::parse_break_loop,
            Signature::GotoLoopStart => Self::parse_goto_loop_start,
            Signature::PrepareTransition => Self::parse_prepare_transition,
            Signature::ExecuteTransition => Self::parse_execute_transition,
            Signature::SetTransition => Self::parse_set_transition,
            Signature::Move => Self::parse_move,
            Signature::WaitForMove => Self::parse_wait_for_move,
            Signature::MoveDuringEventsOn => Self::parse_move_during_events_on,
            Signature::MoveDuringEventsOff => Self::parse_move_during_events_off,
            Signature::GotoTitle => Self::parse_goto_title,
            Signature::GameEnd => Self::parse_game_end,
            Signature::StopNonPictureGraphicUpdates => Self::parse_stop_non_picture_graphic_updates,
            Signature::ResumeNonPictureGraphicUpdates
                => Self::parse_resume_non_picture_graphic_updates,
            Signature::ForceExitEvent => Self::parse_force_exit_event,
            Signature::EraseEvent => Self::parse_erase_event,
            Signature::Wait => Self::parse_wait,
            Signature::LoopCount => Self::parse_loop_count,
            Signature::LabelPoint => Self::parse_label_point,
            Signature::LabelJump => Self::parse_label_jump,

            Signature::CallEvent1 | Signature::CallEvent2 |
            Signature::CallEvent3 | Signature::CallEvent4 |
            Signature::CallEvent5 | Signature::CallEvent6 |
            Signature::CallEvent7 | Signature::CallEventByName1 |
            Signature::CallEventByName2 | Signature::CallEventByName3 |
            Signature::CallEventByName4 | Signature::CallEventByName5 |
            Signature::CallEventByName6 | Signature::CallEventByName7 |
            Signature::CallEventByVariable1 | Signature::CallEventByVariable2
                => Self::parse_call_common_event,

            Signature::ReserveEvent => Self::parse_reserve_common_event,

            Signature::Checkpoint => Self::parse_checkpoint,
            Signature::Exit => Self::parse_exit,
            _ => |_: &[u8], signature: u32| {
                panic!("Unknown command code {:08x}", signature)
            }
        };

        let (bytes_read, commands_read, command): (usize, u32, Self)
            = command_parser(&bytes[offset..], signature);

        (offset + bytes_read, commands + commands_read, command)
    }

    /// Parse raw bytes into a [`Command`] collection.
    ///
    /// Use of this method is highly discouraged unless you know exactly what you are doing.
    /// Prefer using [`Map::parse`] and then extract what you want from the structure tree.
    ///
    /// # Panics
    /// This function will panic if the given bytes do not represent a valid command structure.
    ///
    /// This might be caused by unaligned bytes, corrupt files, incompatible format updates and
    /// library bugs.
    /// If you are confident you are doing everything right, feel free to report an issue on [GitHub].
    ///
    /// [`Map::parse`]: crate::map::Map::parse
    /// [GitHub]: https://github.com/G1org1owo/wolfrpg-map-parser/issues
    pub fn parse_multiple(bytes: &[u8]) -> (usize, u32, Vec<Command>) {
        let mut offset: usize = 0;
        let mut command_count: u32 = 0;
        let mut commands: Vec<Command> = vec![];

        let mut exit: bool = false;

        while !exit {
            let (bytes_read, commands_read, command): (usize, u32, Command)
                = Command::parse(&bytes[offset..]);

            exit = matches!(command, Command::Exit());

            offset += bytes_read;
            command_count += commands_read;
            commands.push(command);
        }

        (offset, command_count, commands)
    }

    fn parse_show_message(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, ShowTextCommand) = ShowTextCommand::parse(bytes);

        (bytes_read, 0, Command::ShowMessage(command))
    }

    fn parse_comment(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, ShowTextCommand) = ShowTextCommand::parse(bytes);

        (bytes_read, 0, Command::Comment(command))
    }

    fn parse_debug_text(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, ShowTextCommand) = ShowTextCommand::parse(bytes);

        (bytes_read, 0, Command::DebugText(command))
    }

    fn parse_force_close_message(_: &[u8], _: u32) -> (usize, u32, Self) {
        Self::parse_empty(Self::ForceCloseMessage())
    }

    fn parse_clear_debug_text(_: &[u8], _: u32) -> (usize, u32, Self) {
        Self::parse_empty(Self::ClearDebugText())
    }

    fn parse_show_choice(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, commands_read, command): (usize, u32, ShowChoiceCommand)
            = ShowChoiceCommand::parse(bytes);

        (bytes_read, commands_read, Command::ShowChoice(command))
    }

    fn parse_set_variable_base(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, SetVariableCommand)
            = SetVariableCommand::parse_base(bytes);

        (bytes_read, 0, Command::SetVariable(command))
    }

    fn parse_set_variable_range(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, SetVariableCommand)
            = SetVariableCommand::parse_range(bytes);

        (bytes_read, 0, Command::SetVariable(command))
    }

    fn parse_set_variable_db(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, SetVariableCommand) 
            = SetVariableCommand::parse_db(bytes);

        (bytes_read, 0, Command::SetVariable(command))
    }

    fn parse_db_management_base(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, DBManagementCommand)
            = DBManagementCommand::parse_base(bytes);

        (bytes_read, 0, Command::DBManagement(command))
    }

    fn parse_db_management_string(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, DBManagementCommand)
            = DBManagementCommand::parse_string(bytes);

        (bytes_read, 0, Command::DBManagement(command))
    }

    fn parse_db_management_csv(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, DBManagementCommand)
            = DBManagementCommand::parse_csv(bytes);

        (bytes_read, 0, Command::DBManagement(command))
    }

    fn parse_set_string_base(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, SetStringCommand) = SetStringCommand::parse_base(bytes);

        (bytes_read, 0, Command::SetString(command))
    }

    fn parse_set_string_dynamic(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, SetStringCommand)
            = SetStringCommand::parse_dynamic(bytes);

        (bytes_read, 0, Command::SetString(command))
    }

    fn parse_set_variable_plus_base(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, SetVariablePlusCommand)
            = SetVariablePlusCommand::parse_base(bytes);

        (bytes_read, 0, Command::SetVariablePlus(command))
    }

    fn parse_set_variable_plus_other(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, SetVariablePlusCommand)
            = SetVariablePlusCommand::parse_other(bytes);

        (bytes_read, 0, Command::SetVariablePlus(command))
    }

    fn parse_number_condition(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, commands_read, command): (usize, u32, NumberConditionCommand)
            = NumberConditionCommand::parse(bytes);

        (bytes_read, commands_read, Command::NumberCondition(command))
    }

    fn parse_string_condition(bytes: &[u8], signature: u32) -> (usize, u32, Self) {
        let (bytes_read, commands_read, command): (usize, u32, StringConditionCommand)
            = StringConditionCommand::parse(bytes, signature);

        (bytes_read, commands_read, Command::StringCondition(command))
    }

    fn parse_input_key_base(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, InputKeyCommand)
            = InputKeyCommand::parse_input_key_base(bytes);

        (bytes_read, 0, Command::InputKey(command))
    }

    fn parse_input_key_keyboard_or_pad(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, InputKeyCommand)
            = InputKeyCommand::parse_input_key_keyboard_or_pad(bytes);

        (bytes_read, 0, Command::InputKey(command))
    }

    fn parse_automatic_input_base(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, InputKeyCommand)
            = InputKeyCommand::parse_automatic_input_base(bytes);

        (bytes_read, 0, Command::InputKey(command))
    }

    fn parse_automatic_input_keyboard(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, InputKeyCommand)
            = InputKeyCommand::parse_automatic_input_keyboard(bytes);

        (bytes_read, 0, Command::InputKey(command))
    }

    fn parse_input_toggle(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, InputKeyCommand)
            = InputKeyCommand::parse_input_toggle(bytes);

        (bytes_read, 0, Command::InputKey(command))
    }

    fn parse_picture_show_base(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, PictureCommand)
            = PictureCommand::parse_show_base(bytes);

        (bytes_read, 0, Command::Picture(command))
    }

    fn parse_picture_show_colors(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, PictureCommand)
            = PictureCommand::parse_show_colors(bytes);

        (bytes_read, 0, Command::Picture(command))
    }

    fn parse_picture_show_delay(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, PictureCommand)
            = PictureCommand::parse_show_delay(bytes);

        (bytes_read, 0, Command::Picture(command))
    }

    fn parse_picture_show_range(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, PictureCommand)
            = PictureCommand::parse_show_range(bytes);

        (bytes_read, 0, Command::Picture(command))
    }

    fn parse_picture_show_color_values(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, PictureCommand)
            = PictureCommand::parse_color_values(bytes);

        (bytes_read, 0, Command::Picture(command))
    }

    fn parse_picture_show_zoom(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, PictureCommand)
            = PictureCommand::parse_show_zoom(bytes);

        (bytes_read, 0, Command::Picture(command))
    }

    fn parse_picture_show_free_transform(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, PictureCommand)
            = PictureCommand::parse_show_free_transform(bytes);

        (bytes_read, 0, Command::Picture(command))
    }

    fn parse_picture_erase_delay_reset(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, PictureCommand)
            = PictureCommand::parse_erase_delay_reset(bytes);

        (bytes_read, 0, Command::Picture(command))
    }

    fn parse_picture_erase_base(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, PictureCommand)
            = PictureCommand::parse_erase_base(bytes);

        (bytes_read, 0, Command::Picture(command))
    }

    fn parse_picture_erase_delay(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, PictureCommand)
            = PictureCommand::parse_erase_delay(bytes);

        (bytes_read, 0, Command::Picture(command))
    }

    fn parse_picture_erase_range(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, PictureCommand)
            = PictureCommand::parse_erase_range(bytes);

        (bytes_read, 0, Command::Picture(command))
    }

    fn parse_effect_base(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, EffectCommand)
            = EffectCommand::parse_base(bytes);

        (bytes_read, 0, Command::Effect(command))
    }

    fn parse_effect_map_shake(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, EffectCommand)
            = EffectCommand::parse_map_shake(bytes);

        (bytes_read, 0, Command::Effect(command))
    }

    fn parse_effect_scroll_screen(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, EffectCommand)
            = EffectCommand::parse_scroll_screen(bytes);

        (bytes_read, 0, Command::Effect(command))
    }

    fn parse_effect_change_color(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, EffectCommand)
            = EffectCommand::parse_change_color(bytes);

        (bytes_read, 0, Command::Effect(command))
    }

    fn parse_sound_filename(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, SoundCommand)
            = SoundCommand::parse_filename(bytes);

        (bytes_read, 0, Command::Sound(command))
    }

    fn parse_sound_variable(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, SoundCommand)
            = SoundCommand::parse_variable(bytes);

        (bytes_read, 0, Command::Sound(command))
    }

    fn parse_sound_free_all(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, SoundCommand)
            = SoundCommand::parse_free_all(bytes);

        (bytes_read, 0, Command::Sound(command))
    }

    fn parse_save_load_base(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, SaveLoadCommand)
            = SaveLoadCommand::parse_base(bytes);

        (bytes_read, 0, Command::SaveLoad(command))
    }

    fn parse_load_variable(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, SaveLoadCommand)
            = SaveLoadCommand::parse_load_variable(bytes);

        (bytes_read, 0, Command::SaveLoad(command))
    }

    fn parse_save_variable(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, SaveLoadCommand)
            = SaveLoadCommand::parse_save_variable(bytes);

        (bytes_read, 0, Command::SaveLoad(command))
    }

    fn parse_party_graphics(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, PartyGraphicsCommand)
            = PartyGraphicsCommand::parse(bytes);

        (bytes_read, 0, Command::PartyGraphics(command))
    }

    fn parse_chip_management_settings(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, ChipManagementCommand)
            = ChipManagementCommand::parse_map_chip_settings(bytes);

        (bytes_read, 0, Command::ChipManagement(command))
    }

    fn parse_chip_management_switch_set(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, ChipManagementCommand)
            = ChipManagementCommand::parse_switch_chipset(bytes);

        (bytes_read, 0, Command::ChipManagement(command))
    }

    fn parse_chip_management_overwrite(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, ChipManagementCommand)
            = ChipManagementCommand::parse_overwrite_map_chips(bytes);

        (bytes_read, 0, Command::ChipManagement(command))
    }

    fn parse_transfer(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, TransferCommand)
            = TransferCommand::parse(bytes);

        (bytes_read, 0, Command::Transfer(command))
    }

    fn parse_loop(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, commands_read, command): (usize, u32, EventControlCommand)
            = EventControlCommand::parse_loop(bytes);

        (bytes_read, commands_read, Command::EventControl(command))
    }

    fn parse_break_loop(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, EventControlCommand)
            = EventControlCommand::parse_break_loop(bytes);

        (bytes_read, 0, Command::EventControl(command))
    }

    fn parse_goto_loop_start(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, EventControlCommand)
            = EventControlCommand::parse_goto_loop_start(bytes);

        (bytes_read, 0, Command::EventControl(command))
    }

    fn parse_prepare_transition(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, EventControlCommand)
            = EventControlCommand::parse_prepare_transition(bytes);

        (bytes_read, 0, Command::EventControl(command))
    }

    fn parse_execute_transition(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, EventControlCommand)
            = EventControlCommand::parse_execute_transition(bytes);

        (bytes_read, 0, Command::EventControl(command))
    }

    fn parse_set_transition(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, EventControlCommand)
            = EventControlCommand::parse_set_transition(bytes);

        (bytes_read, 0, Command::EventControl(command))
    }

    fn parse_move(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, EventControlCommand)
            = EventControlCommand::parse_move_route(bytes);

        (bytes_read, 0, Command::EventControl(command))
    }

    fn parse_wait_for_move(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, EventControlCommand)
            = EventControlCommand::parse_wait_for_move_route(bytes);

        (bytes_read, 0, Command::EventControl(command))
    }

    fn parse_move_during_events_on(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, EventControlCommand)
            = EventControlCommand::parse_move_during_events_on(bytes);

        (bytes_read, 0, Command::EventControl(command))
    }

    fn parse_move_during_events_off(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, EventControlCommand)
            = EventControlCommand::parse_move_during_events_off(bytes);

        (bytes_read, 0, Command::EventControl(command))
    }

    fn parse_goto_title(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, EventControlCommand)
            = EventControlCommand::parse_goto_title(bytes);

        (bytes_read, 0, Command::EventControl(command))
    }

    fn parse_game_end(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, EventControlCommand)
            = EventControlCommand::parse_game_end(bytes);

        (bytes_read, 0, Command::EventControl(command))
    }

    fn parse_stop_non_picture_graphic_updates(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, EventControlCommand)
            = EventControlCommand::parse_stop_non_picture_graphic_updates(bytes);

        (bytes_read, 0, Command::EventControl(command))
    }

    fn parse_resume_non_picture_graphic_updates(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, EventControlCommand)
            = EventControlCommand::parse_resume_non_picture_graphic_updates(bytes);

        (bytes_read, 0, Command::EventControl(command))
    }

    fn parse_force_exit_event(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, EventControlCommand)
            = EventControlCommand::parse_force_exit_event(bytes);

        (bytes_read, 0, Command::EventControl(command))
    }

    fn parse_erase_event(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, EventControlCommand)
            = EventControlCommand::parse_erase_event(bytes);

        (bytes_read, 0, Command::EventControl(command))
    }

    fn parse_wait(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, EventControlCommand)
            = EventControlCommand::parse_wait(bytes);

        (bytes_read, 0, Command::EventControl(command))
    }

    fn parse_loop_count(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, commands_read, command): (usize, u32, EventControlCommand)
            = EventControlCommand::parse_loop_count(bytes);

        (bytes_read, commands_read, Command::EventControl(command))
    }

    fn parse_label_point(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, EventControlCommand)
            = EventControlCommand::parse_label_point(bytes);

        (bytes_read, 0, Command::EventControl(command))
    }

    fn parse_label_jump(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, EventControlCommand)
            = EventControlCommand::parse_label_jump(bytes);

        (bytes_read, 0, Command::EventControl(command))
    }

    fn parse_call_common_event(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, CommonEventCommand)
            = CommonEventCommand::parse_call_event(bytes);

        (bytes_read, 0, Command::CommonEvent(command))
    }

    fn parse_reserve_common_event(bytes: &[u8], _: u32) -> (usize, u32, Self) {
        let (bytes_read, command): (usize, CommonEventCommand)
            = CommonEventCommand::parse_reserve_event(bytes);

        (bytes_read, 0, Command::CommonEvent(command))
    }

    fn parse_checkpoint(_: &[u8], _: u32) -> (usize, u32, Self) {
        (7, 0, Self::Checkpoint())
    }

    fn parse_exit(_: &[u8], _: u32) -> (usize, u32, Self) {
        Self::parse_empty(Self::Exit())
    }

    fn parse_empty(ret: Self) -> (usize, u32, Self) {
        (3, 0, ret)
    }
}
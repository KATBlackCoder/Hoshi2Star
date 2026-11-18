//! The texts of the Wolf event commands that hold messages, and their
//! segment keys, shared by the extractor and the injector (plan DD, L3):
//! Show Message (101) holds one text, its first string; Choices (102) one
//! text per string. Keys are `{prefix}/{cmd}` and `{prefix}/{cmd}/choices/{i}`.
//! `integrity.rs` reads the same commands on its own, on purpose: it checks
//! the injector's output independently of it.

use super::v3_format::command::{Command, CID_CHOICES, CID_MESSAGE};

/// Where a text sits in its command: `None` for a message, `Some(i)` for
/// choice `i`.
pub(crate) type Site = Option<usize>;

/// The texts of `command`, in order (none for other commands).
pub(crate) fn texts(command: &Command) -> impl Iterator<Item = (Site, &String)> {
    let (message, choices): (Option<&String>, &[String]) = match command.cid {
        CID_MESSAGE => (command.string_args.first(), &[]),
        CID_CHOICES => (None, &command.string_args),
        _ => (None, &[]),
    };
    message
        .map(|text| (None, text))
        .into_iter()
        .chain(choices.iter().enumerate().map(|(i, text)| (Some(i), text)))
}

/// [`texts`], to rewrite them.
pub(crate) fn texts_mut(command: &mut Command) -> impl Iterator<Item = (Site, &mut String)> {
    let (message, choices): (Option<&mut String>, &mut [String]) = match command.cid {
        CID_MESSAGE => (command.string_args.first_mut(), &mut []),
        CID_CHOICES => (None, &mut command.string_args),
        _ => (None, &mut []),
    };
    message.map(|text| (None, text)).into_iter().chain(
        choices
            .iter_mut()
            .enumerate()
            .map(|(i, text)| (Some(i), text)),
    )
}

/// The segment key of the text at `site` of command `cmd_idx` under `prefix`
/// (`MapData/{map}/events/{e}/pages/{p}` or `CommonEvents/{name}/{e}`).
pub(crate) fn key(prefix: &str, cmd_idx: usize, site: Site) -> String {
    match site {
        None => format!("{prefix}/{cmd_idx}"),
        Some(choice_idx) => format!("{prefix}/{cmd_idx}/choices/{choice_idx}"),
    }
}

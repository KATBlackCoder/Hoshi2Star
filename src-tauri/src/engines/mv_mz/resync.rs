//! Realignment of an MV/MZ project after a game update (lot R2b).
//!
//! A game update that inserts or removes an event command shifts every later
//! JSON pointer of the page, so recorded segments no longer sit at their key.
//! [`align`] pairs the recorded lines of one file with the lines extracted
//! from the new version, so each translation follows its line:
//! - inside an event command list (`…/list/<n>/parameters/…`), lines are
//!   paired per list by the longest common subsequence of `(source, kind)`,
//!   like a `diff`;
//! - any other pointer (database field, map name, note tag, `plugins.js`)
//!   keeps its key: same key and same source, or no pair.
//!
//! A line whose source changed is never paired: it is a new line, and the old
//! one is gone (its translation goes to the translation memory).

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

/// `<list>/<n>/parameters/…`: a value inside an event command.
static RE_COMMAND_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(.*/list)/(\d+)/parameters/").expect("RE_COMMAND_LINE must compile")
});

/// Above this many cells, the exact LCS table is not built (see [`pair_list`]).
const MAX_TABLE_CELLS: usize = 16_000_000;

/// One line of a file: a recorded segment or a freshly extracted one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub key: String,
    pub source: String,
    /// `segments.segment_kind`; `"unknown"` (rows from before 0007) pairs with
    /// any kind.
    pub kind: String,
}

impl Line {
    pub fn new(key: impl Into<String>, source: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            source: source.into(),
            kind: kind.into(),
        }
    }

    fn same_text(&self, other: &Line) -> bool {
        self.source == other.source
            && (self.kind == other.kind || self.kind == "unknown" || other.kind == "unknown")
    }
}

/// How the old lines of a file map onto the new ones (indices into the two
/// slices given to [`align`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Alignment {
    /// `(old, new)` pairs: the old line now lives at the new line's key.
    pub paired: Vec<(usize, usize)>,
    /// New lines with no old counterpart (new or changed text).
    pub added: Vec<usize>,
    /// Old lines with no new counterpart (removed or changed text).
    pub gone: Vec<usize>,
}

impl Alignment {
    /// Pairs whose key changed.
    pub fn moved<'a>(
        &'a self,
        old: &'a [Line],
        new: &'a [Line],
    ) -> impl Iterator<Item = (usize, usize)> + 'a {
        self.paired
            .iter()
            .copied()
            .filter(move |&(o, n)| old[o].key != new[n].key)
    }

    /// Gone lines whose key is still extracted with another text: the source
    /// changed in place.
    pub fn changed_sources(&self, old: &[Line], new: &[Line]) -> usize {
        let added_keys: std::collections::HashSet<&str> =
            self.added.iter().map(|&n| new[n].key.as_str()).collect();
        self.gone
            .iter()
            .filter(|&&o| added_keys.contains(old[o].key.as_str()))
            .count()
    }

    pub fn is_unchanged(&self, old: &[Line], new: &[Line]) -> bool {
        self.added.is_empty() && self.gone.is_empty() && self.moved(old, new).next().is_none()
    }
}

/// `(command list pointer, command index)` of a line inside an event command.
fn command_position(key: &str) -> Option<(&str, usize)> {
    let captures = RE_COMMAND_LINE.captures(key)?;
    let list = captures.get(1)?.as_str();
    let index = captures.get(2)?.as_str().parse().ok()?;
    Some((list, index))
}

/// Pair the recorded lines of one file (`old`, in extraction order) with the
/// lines extracted from its new version (`new`, in extraction order).
pub fn align(old: &[Line], new: &[Line]) -> Alignment {
    let mut lists_old: HashMap<&str, Vec<usize>> = HashMap::new();
    let mut lists_new: HashMap<&str, Vec<usize>> = HashMap::new();
    let mut fixed_new: HashMap<&str, usize> = HashMap::new();
    let mut fixed_old = Vec::new();
    for (index, line) in old.iter().enumerate() {
        match command_position(&line.key) {
            Some((list, _)) => lists_old.entry(list).or_default().push(index),
            None => fixed_old.push(index),
        }
    }
    for (index, line) in new.iter().enumerate() {
        match command_position(&line.key) {
            Some((list, _)) => lists_new.entry(list).or_default().push(index),
            None => {
                fixed_new.insert(line.key.as_str(), index);
            }
        }
    }

    let mut paired = Vec::new();
    for o in fixed_old {
        if let Some(&n) = fixed_new.get(old[o].key.as_str()) {
            if old[o].source == new[n].source {
                paired.push((o, n));
            }
        }
    }
    for (list, mut old_lines) in lists_old {
        let Some(new_lines) = lists_new.get(list) else {
            continue;
        };
        // Command order, then extraction order within a command (choices).
        old_lines.sort_by_key(|&o| command_position(&old[o].key).map(|(_, index)| index));
        paired.extend(pair_list(&old_lines, new_lines, |o, n| {
            old[o].same_text(&new[n])
        }));
    }

    paired.sort_unstable();
    let paired_old: std::collections::HashSet<usize> = paired.iter().map(|&(o, _)| o).collect();
    let paired_new: std::collections::HashSet<usize> = paired.iter().map(|&(_, n)| n).collect();
    Alignment {
        added: (0..new.len()).filter(|n| !paired_new.contains(n)).collect(),
        gone: (0..old.len()).filter(|o| !paired_old.contains(o)).collect(),
        paired,
    }
}

/// Longest common subsequence of two line sequences under `same`. Common
/// prefix and suffix are paired first (a single insertion costs nothing);
/// a middle too large for the exact table falls back to an in-order greedy
/// pairing, still a valid common subsequence.
fn pair_list(
    old: &[usize],
    new: &[usize],
    same: impl Fn(usize, usize) -> bool,
) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    let mut start = 0;
    while start < old.len() && start < new.len() && same(old[start], new[start]) {
        pairs.push((old[start], new[start]));
        start += 1;
    }
    let (mut old_end, mut new_end) = (old.len(), new.len());
    let mut suffix = Vec::new();
    while old_end > start && new_end > start && same(old[old_end - 1], new[new_end - 1]) {
        old_end -= 1;
        new_end -= 1;
        suffix.push((old[old_end], new[new_end]));
    }
    let (a, b) = (&old[start..old_end], &new[start..new_end]);

    if !a.is_empty() && !b.is_empty() {
        let fits = a.len() < usize::from(u16::MAX)
            && b.len() < usize::from(u16::MAX)
            && (a.len() + 1) * (b.len() + 1) <= MAX_TABLE_CELLS;
        if fits {
            // table[i][j] = LCS length of a[i..] and b[j..].
            let width = b.len() + 1;
            let mut table = vec![0u16; (a.len() + 1) * width];
            for i in (0..a.len()).rev() {
                for j in (0..b.len()).rev() {
                    table[i * width + j] = if same(a[i], b[j]) {
                        table[(i + 1) * width + j + 1] + 1
                    } else {
                        table[(i + 1) * width + j].max(table[i * width + j + 1])
                    };
                }
            }
            let (mut i, mut j) = (0, 0);
            while i < a.len() && j < b.len() {
                if same(a[i], b[j]) {
                    pairs.push((a[i], b[j]));
                    i += 1;
                    j += 1;
                } else if table[(i + 1) * width + j] >= table[i * width + j + 1] {
                    i += 1;
                } else {
                    j += 1;
                }
            }
        } else {
            let mut j = 0;
            for &o in a {
                if let Some(offset) = b[j..].iter().position(|&n| same(o, n)) {
                    pairs.push((o, b[j + offset]));
                    j += offset + 1;
                }
            }
        }
    }
    pairs.extend(suffix.into_iter().rev());
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(index: usize, parameter: &str, source: &str, kind: &str) -> Line {
        Line::new(
            format!("/events/1/pages/0/list/{index}/parameters/{parameter}"),
            source,
            kind,
        )
    }

    #[test]
    fn an_inserted_line_moves_every_later_translation() {
        let old = vec![
            line(1, "0", "おはよう", "dialogue"),
            line(3, "0", "こんばんは", "dialogue"),
        ];
        let new = vec![
            line(1, "0", "おはよう", "dialogue"),
            line(2, "0", "新しい台詞", "dialogue"),
            line(4, "0", "こんばんは", "dialogue"),
        ];
        let alignment = align(&old, &new);
        assert_eq!(alignment.paired, vec![(0, 0), (1, 2)]);
        assert_eq!(alignment.added, vec![1]);
        assert!(alignment.gone.is_empty());
        assert_eq!(
            alignment.moved(&old, &new).collect::<Vec<_>>(),
            vec![(1, 2)]
        );
    }

    #[test]
    fn removed_and_changed_lines_are_gone_and_never_paired_approximately() {
        let old = vec![
            line(1, "0", "一", "dialogue"),
            line(2, "0", "二", "dialogue"),
            line(3, "0", "三", "dialogue"),
        ];
        // "二" removed, "三" rewritten in place.
        let new = vec![
            line(1, "0", "一", "dialogue"),
            line(2, "0", "三です", "dialogue"),
        ];
        let alignment = align(&old, &new);
        assert_eq!(alignment.paired, vec![(0, 0)]);
        assert_eq!(alignment.gone, vec![1, 2]);
        assert_eq!(alignment.added, vec![1]);
        // `…/list/2/parameters/0` is extracted again with another text.
        assert_eq!(alignment.changed_sources(&old, &new), 1);
    }

    #[test]
    fn same_text_of_another_kind_is_not_paired() {
        let old = vec![line(1, "0", "はい", "dialogue")];
        let new = vec![line(1, "0/0", "はい", "choice")];
        let alignment = align(&old, &new);
        assert!(alignment.paired.is_empty());
        // Rows from before segment kinds existed pair with any kind.
        let legacy = vec![line(1, "0", "はい", "unknown")];
        assert_eq!(align(&legacy, &new).paired, vec![(0, 0)]);
    }

    #[test]
    fn reordered_page_keeps_the_longest_common_order_and_repeats_pair_in_order() {
        let old = vec![
            line(0, "0", "A", "dialogue"),
            line(1, "0", "B", "dialogue"),
            line(2, "0", "A", "dialogue"),
            line(3, "0", "C", "dialogue"),
        ];
        let new = vec![
            line(0, "0", "C", "dialogue"),
            line(1, "0", "A", "dialogue"),
            line(2, "0", "B", "dialogue"),
            line(3, "0", "A", "dialogue"),
        ];
        let alignment = align(&old, &new);
        assert_eq!(alignment.paired, vec![(0, 1), (1, 2), (2, 3)]);
        assert_eq!(alignment.gone, vec![3]);
        assert_eq!(alignment.added, vec![0]);
    }

    #[test]
    fn lists_are_aligned_independently_and_fixed_keys_need_the_same_source() {
        let old = vec![
            Line::new("/1/name", "勇者", "actor_name"),
            Line::new("/2/name", "魔王", "actor_name"),
            Line::new("/events/1/pages/0/list/0/parameters/0", "やあ", "dialogue"),
            Line::new("/events/2/pages/0/list/0/parameters/0", "やあ", "dialogue"),
        ];
        let new = vec![
            Line::new("/1/name", "勇者", "actor_name"),
            Line::new("/2/name", "大魔王", "actor_name"),
            Line::new("/events/2/pages/0/list/1/parameters/0", "やあ", "dialogue"),
        ];
        let alignment = align(&old, &new);
        // Event 1's line does not jump to event 2.
        assert_eq!(alignment.paired, vec![(0, 0), (3, 2)]);
        assert_eq!(alignment.gone, vec![1, 2]);
        assert_eq!(alignment.added, vec![1]);
    }

    #[test]
    fn a_middle_too_large_for_the_table_still_pairs_in_order() {
        let same = |o: usize, n: usize| o % 7 == n % 7;
        let old: Vec<usize> = (0..5000).collect();
        let new: Vec<usize> = (1..5001).collect();
        let pairs = pair_list(&old, &new, same);
        assert!(!pairs.is_empty());
        assert!(pairs.windows(2).all(|w| w[0].0 < w[1].0 && w[0].1 < w[1].1));
        assert!(pairs.iter().all(|&(o, n)| same(o, n)));
    }
}

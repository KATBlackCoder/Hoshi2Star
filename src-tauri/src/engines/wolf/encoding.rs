// Wolf RPG — Shift-JIS ↔ UTF-8 conversion layer.
//
// Wolf RPG v1/v2 files use Shift-JIS (code page 932).
// Wolf RPG v3+ files use UTF-8.
// This module centralises encoding logic to prevent drift between extractor and injector.

use std::borrow::Cow;
use std::path::Path;

use encoding_rs::SHIFT_JIS;
use unicode_normalization::char::{decompose_canonical, is_combining_mark};

/// Decode Shift-JIS bytes to a UTF-8 `String`.
pub(crate) fn decode_shiftjis(bytes: &[u8]) -> Result<String, String> {
    let (decoded, _, had_errors) = SHIFT_JIS.decode(bytes);
    if had_errors {
        Err(format!("Shift-JIS decode error on {} bytes", bytes.len()))
    } else {
        Ok(decoded.into_owned())
    }
}

/// Encode a UTF-8 `str` to Shift-JIS bytes.
///
/// Returns `Err` if any character cannot be represented in Shift-JIS (e.g. emoji,
/// Latin letters with diacritics beyond the limited SJIS range). Used by F4-04
/// injector as a guard — Wolf RPG v2 will crash if UTF-8 bytes are written back.
// Used by the injector (F4-04); unused in F4-03 extractor.
#[allow(dead_code)]
pub(crate) fn encode_shiftjis(text: &str) -> Result<Vec<u8>, String> {
    let (encoded, _, had_errors) = SHIFT_JIS.encode(text);
    if had_errors {
        Err(format!(
            "Shift-JIS encode error: '{}' contains characters not in Shift-JIS",
            text
        ))
    } else {
        Ok(encoded.into_owned())
    }
}

/// `true` when every character of `text` has a Shift-JIS encoding.
pub(crate) fn is_shift_jis(text: &str) -> bool {
    !SHIFT_JIS.encode(text).2
}

/// `text` as a Shift-JIS Wolf file can hold it (plan RW, decision D3): a
/// character Shift-JIS cannot encode becomes its base letter (é → e, Ç → C),
/// ligatures are spelled out (œ → oe, ß → ss), typographic quotes, spaces and
/// dashes take their plain form. Any other character is kept, so the encoder
/// still rejects it with an explicit error. Borrowed when nothing changes.
pub(crate) fn fold_for_shift_jis(text: &str) -> Cow<'_, str> {
    if is_shift_jis(text) {
        return Cow::Borrowed(text);
    }
    let mut folded = String::with_capacity(text.len());
    let mut buf = [0u8; 4];
    for c in text.chars() {
        if is_shift_jis(c.encode_utf8(&mut buf)) {
            folded.push(c);
            continue;
        }
        match c {
            'œ' => folded.push_str("oe"),
            'Œ' => folded.push_str("OE"),
            'æ' => folded.push_str("ae"),
            'Æ' => folded.push_str("AE"),
            'ß' => folded.push_str("ss"),
            '«' | '»' | '„' => folded.push('"'),
            '‚' => folded.push('\''),
            '\u{00A0}' | '\u{2009}' | '\u{202F}' => folded.push(' '),
            '–' | '—' => folded.push('-'),
            _ => {
                let mut base = String::new();
                decompose_canonical(c, |part| {
                    if !is_combining_mark(part) {
                        base.push(part);
                    }
                });
                if !base.is_empty() && is_shift_jis(&base) {
                    folded.push_str(&base);
                } else {
                    folded.push(c);
                }
            }
        }
    }
    Cow::Owned(folded)
}

/// Characters `encoding_rs` (WHATWG Shift_JIS) encodes, but whose Windows
/// code page 932 round trip yields another character (0x8160 is U+301C for
/// WHATWG, U+FF5E for Windows; likewise 0x8161, 0x817C, 0x8191, 0x8192,
/// 0x81CA). A path holding one of them no longer matches itself once a
/// Shift-JIS program converts it.
const CP932_ROUND_TRIP_MISMATCH: [char; 6] = [
    '\u{301C}', '\u{2016}', '\u{2212}', '\u{00A2}', '\u{00A3}', '\u{00AC}',
];

/// Warning for a Shift-JIS (Wolf 2.x) game whose folder path a Shift-JIS
/// program cannot keep intact: a character without a code page 932 encoding,
/// or one whose round trip changes it (`〜` U+301C in the real case: the
/// Wolf 2.23 test game did not start from its original folder, it did from an
/// ASCII copy). `None` otherwise.
pub(crate) fn path_warning(
    game_dir: &Path,
    version: &crate::engines::detector::WolfVersion,
) -> Option<String> {
    if version.is_utf8() {
        return None;
    }
    let path = game_dir.to_string_lossy();
    let unsafe_path =
        !is_shift_jis(&path) || path.chars().any(|c| CP932_ROUND_TRIP_MISMATCH.contains(&c));
    unsafe_path.then(|| {
        format!(
            "this Wolf RPG 2.x game may not start from a folder whose path holds \
             characters a Shift-JIS program cannot keep (such as 〜 or accented \
             letters); move it to a path with only ASCII characters ({path})"
        )
    })
}

/// Select encoding based on Wolf RPG version.
///
/// - v3+ → UTF-8 (native)
/// - v1/v2 → Shift-JIS decode via `encoding_rs`
// Used by higher-level parsers in F4-04/05; direct SJIS decode used in F4-03.
#[allow(dead_code)]
pub(crate) fn decode_wolf_text(
    bytes: &[u8],
    version: &crate::engines::detector::WolfVersion,
) -> Result<String, String> {
    if version.is_utf8() {
        String::from_utf8(bytes.to_vec()).map_err(|e| format!("UTF-8 decode error: {e}"))
    } else {
        decode_shiftjis(bytes)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_shiftjis_ascii() {
        let result = decode_shiftjis(b"Hello").unwrap();
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_decode_shiftjis_japanese() {
        // "テスト" in Shift-JIS: テ=0x83 0x65, ス=0x83 0x58, ト=0x83 0x67
        let sjis_bytes: &[u8] = &[0x83, 0x65, 0x83, 0x58, 0x83, 0x67];
        let result = decode_shiftjis(sjis_bytes).unwrap();
        assert_eq!(result, "テスト");
    }

    #[test]
    fn test_encode_shiftjis_ascii() {
        let result = encode_shiftjis("Hello").unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_fold_for_shift_jis() {
        assert_eq!(
            fold_for_shift_jis("Élève, ça déçoit à Noël : « là-bas » — très bien."),
            "Eleve, ca decoit a Noel : \" la-bas \" - tres bien."
        );
        assert_eq!(
            fold_for_shift_jis("Œuvre, cœur, Straße"),
            "OEuvre, coeur, Strasse"
        );
        assert_eq!(fold_for_shift_jis("prix\u{00A0}: 3\u{202F}%"), "prix : 3 %");
        // Already encodable: borrowed, untouched (Japanese, ellipsis, fullwidth).
        for text in ["こんにちは、旅の人", "Hello…", "×２"] {
            assert!(
                matches!(fold_for_shift_jis(text), Cow::Borrowed(_)),
                "{text}"
            );
        }
        // No Shift-JIS base form: kept, so the encoder still refuses it.
        assert_eq!(fold_for_shift_jis("ok 😀"), "ok 😀");
        assert!(encode_shiftjis(&fold_for_shift_jis("ok 😀")).is_err());
    }

    #[test]
    fn test_path_warning_for_v2_games_only() {
        use crate::engines::detector::WolfVersion;
        let v2 = WolfVersion { major: 2, minor: 0 };
        let v3 = WolfVersion { major: 3, minor: 0 };
        let wave_dash = Path::new("/home/user/jeux/海と川\u{301C}体験版");
        assert!(path_warning(wave_dash, &v2).is_some());
        assert!(path_warning(wave_dash, &v3).is_none());
        assert!(path_warning(Path::new("/home/user/jeux/海と川の体験版"), &v2).is_none());
        assert!(path_warning(Path::new("/home/user/jeux/W0"), &v2).is_none());
        assert!(path_warning(Path::new("/home/user/jeux/élève"), &v2).is_some());
    }

    #[test]
    fn test_encode_shiftjis_accented() {
        // "café" — é (U+00E9) is not representable in Shift-JIS
        let result = encode_shiftjis("café");
        assert!(
            result.is_err(),
            "should fail for characters outside Shift-JIS"
        );
    }
}

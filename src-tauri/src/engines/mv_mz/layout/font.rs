//! Game font metrics.
//!
//! RPG Maker MZ draws message text one character at a time in normal play
//! (`Window_Message.updateMessage` flushes after every character), so a line's
//! width is the sum of each character's advance at the current font size.
//! Fonts ship as TTF/OTF or WOFF1 (`mplus-1m-regular.woff` by default).

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FontError {
    #[error("font file is truncated or malformed: {0}")]
    Malformed(&'static str),
    #[error("WOFF2 fonts are not supported")]
    Woff2,
    #[error("font table could not be decompressed: {0}")]
    Inflate(String),
    #[error("font could not be parsed: {0}")]
    Parse(String),
    #[error("font file could not be read: {0}")]
    Io(String),
}

/// Per-character advances of a font, in font units.
#[derive(Debug, Clone)]
pub struct GameFont {
    units_per_em: f32,
    advances: HashMap<char, u16>,
}

impl GameFont {
    /// Load a TTF/OTF or WOFF1 font file.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, FontError> {
        let sfnt = match bytes.get(..4) {
            Some(b"wOFF") => woff1_to_sfnt(bytes)?,
            Some(b"wOF2") => return Err(FontError::Woff2),
            Some(_) => bytes.to_vec(),
            None => return Err(FontError::Malformed("empty file")),
        };
        let face =
            ttf_parser::Face::parse(&sfnt, 0).map_err(|e| FontError::Parse(e.to_string()))?;
        let mut advances = HashMap::new();
        if let Some(cmap) = face.tables().cmap {
            for subtable in cmap.subtables {
                if !subtable.is_unicode() {
                    continue;
                }
                subtable.codepoints(|code_point| {
                    let Some(ch) = char::from_u32(code_point) else {
                        return;
                    };
                    if advances.contains_key(&ch) {
                        return;
                    }
                    if let Some(advance) = subtable
                        .glyph_index(code_point)
                        .and_then(|glyph| face.glyph_hor_advance(glyph))
                    {
                        advances.insert(ch, advance);
                    }
                });
            }
        }
        if advances.is_empty() {
            return Err(FontError::Parse("no Unicode character map".into()));
        }
        Ok(Self {
            units_per_em: f32::from(face.units_per_em().max(1)),
            advances,
        })
    }

    /// Advance of `ch` at `size` px, or `None` when the font has no glyph
    /// for it (the browser then draws it with a fallback font).
    pub fn advance(&self, ch: char, size: f32) -> Option<f32> {
        self.advances
            .get(&ch)
            .map(|units| f32::from(*units) * size / self.units_per_em)
    }
}

/// Character widths used for layout: the game's font when it could be read,
/// otherwise an estimate modelled on RPG Maker's default M+ 1m font
/// (half-width = ½ em, full-width and East Asian ambiguous = 1 em).
#[derive(Debug, Clone)]
pub enum Metrics {
    Font(Arc<GameFont>),
    Estimated,
}

impl Metrics {
    /// Width of `ch` at `size` px and whether it was measured from the font.
    pub fn char_width(&self, ch: char, size: f32) -> (f32, bool) {
        match self {
            Metrics::Font(font) => match font.advance(ch, size) {
                Some(width) => (width, true),
                // Fallback fonts (e.g. Verdana) are proportional Latin fonts:
                // wide characters stay 1 em, others average ~0.6 em.
                None if is_wide(ch) => (size, false),
                None => (size * 0.6, false),
            },
            Metrics::Estimated if is_wide(ch) || is_ambiguous(ch) => (size, false),
            Metrics::Estimated => (size * 0.5, false),
        }
    }

    /// Sum of per-character widths (how normal play draws message text).
    pub fn text_width(&self, text: &str, size: f32) -> f32 {
        text.chars().map(|ch| self.char_width(ch, size).0).sum()
    }

    /// Characters of `text` the game font has no glyph for.
    pub fn missing_chars(&self, text: &str) -> Vec<char> {
        match self {
            Metrics::Font(font) => text
                .chars()
                .filter(|ch| !ch.is_control() && font.advance(*ch, 1.0).is_none())
                .collect(),
            Metrics::Estimated => Vec::new(),
        }
    }
}

/// East Asian Wide / Fullwidth characters (UAX #11, main ranges).
pub fn is_wide(ch: char) -> bool {
    matches!(ch as u32,
        0x1100..=0x115F
            | 0x2E80..=0x303E
            | 0x3041..=0x33FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xA000..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE30..=0xFE4F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x1F300..=0x1F64F
            | 0x1F900..=0x1F9FF
            | 0x20000..=0x3FFFD)
}

/// East Asian Ambiguous punctuation and symbols that Japanese fonts draw
/// full-width (`…`, `—`, `♥`, `※`, curly quotes…).
fn is_ambiguous(ch: char) -> bool {
    matches!(ch as u32,
        0x00A7 | 0x00A8 | 0x00B0 | 0x00B1 | 0x00B4 | 0x00B6 | 0x00D7 | 0x00F7
            | 0x2010 | 0x2013..=0x2016 | 0x2018 | 0x2019 | 0x201C | 0x201D
            | 0x2020..=0x2022 | 0x2025..=0x2027 | 0x2030 | 0x2032 | 0x2033 | 0x203B
            | 0x2190..=0x21FF | 0x2200..=0x22FF | 0x2460..=0x24FF | 0x2500..=0x257F
            | 0x25A0..=0x25FF | 0x2605 | 0x2606 | 0x2640 | 0x2642 | 0x2660..=0x266F)
}

/// `(path, size, modified, font)`.
type CachedFont = (PathBuf, u64, Option<SystemTime>, Arc<GameFont>);

/// The last game font read, keyed by path, size and modification time (one
/// project is open at a time; previews would otherwise decode it on every
/// request).
#[derive(Debug, Default)]
pub struct FontCache {
    entry: Mutex<Option<CachedFont>>,
}

impl FontCache {
    pub fn load(&self, path: &Path) -> Result<Arc<GameFont>, FontError> {
        let meta = std::fs::metadata(path).map_err(|e| FontError::Io(e.to_string()))?;
        let key = (path.to_path_buf(), meta.len(), meta.modified().ok());
        let mut entry = self
            .entry
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some((cached_path, len, modified, font)) = entry.as_ref() {
            if (cached_path, len, modified) == (&key.0, &key.1, &key.2) {
                return Ok(Arc::clone(font));
            }
        }
        let bytes = std::fs::read(path).map_err(|e| FontError::Io(e.to_string()))?;
        let font = Arc::new(GameFont::from_bytes(&bytes)?);
        *entry = Some((key.0, key.1, key.2, Arc::clone(&font)));
        Ok(font)
    }
}

// ---------------------------------------------------------------------------
// WOFF1 → sfnt
// ---------------------------------------------------------------------------

fn be_u16(bytes: &[u8], at: usize) -> Result<u16, FontError> {
    bytes
        .get(at..at + 2)
        .map(|b| u16::from_be_bytes([b[0], b[1]]))
        .ok_or(FontError::Malformed("header"))
}

fn be_u32(bytes: &[u8], at: usize) -> Result<u32, FontError> {
    bytes
        .get(at..at + 4)
        .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or(FontError::Malformed("header"))
}

/// Largest font Hoshi2Star unpacks (CJK game fonts weigh a few tens of MB):
/// the sizes in a WOFF header come from the game folder and are not trusted.
const MAX_SFNT_BYTES: usize = 64 * 1024 * 1024;

/// Unpack a WOFF1 file (zlib-compressed tables) into a plain sfnt.
fn woff1_to_sfnt(woff: &[u8]) -> Result<Vec<u8>, FontError> {
    const HEADER: usize = 44;
    const ENTRY: usize = 20;
    let flavor = be_u32(woff, 4)?;
    let count = usize::from(be_u16(woff, 12)?);
    let mut tables = Vec::with_capacity(count);
    let mut total = 0usize;
    for index in 0..count {
        let at = HEADER + index * ENTRY;
        let tag: [u8; 4] = woff
            .get(at..at + 4)
            .and_then(|b| b.try_into().ok())
            .ok_or(FontError::Malformed("table directory"))?;
        let offset = be_u32(woff, at + 4)? as usize;
        let compressed = be_u32(woff, at + 8)? as usize;
        let original = be_u32(woff, at + 12)? as usize;
        let raw = woff
            .get(offset..offset + compressed)
            .ok_or(FontError::Malformed("table data"))?;
        total = total.saturating_add(original);
        if total > MAX_SFNT_BYTES {
            return Err(FontError::Malformed("font too large"));
        }
        let data = if compressed < original {
            let mut out = Vec::with_capacity(original);
            // Never inflate past the declared size (one byte more proves a lie).
            flate2::read::ZlibDecoder::new(raw)
                .take(original as u64 + 1)
                .read_to_end(&mut out)
                .map_err(|e| FontError::Inflate(e.to_string()))?;
            if out.len() != original {
                return Err(FontError::Malformed("table length"));
            }
            out
        } else {
            raw.to_vec()
        };
        tables.push((tag, data));
    }
    Ok(build_sfnt(flavor, &tables))
}

/// Assemble an sfnt from `(tag, data)` tables (checksums are not needed by
/// the parser and are left at zero).
pub(crate) fn build_sfnt(flavor: u32, tables: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let count = tables.len() as u16;
    let mut search_range = 1u16;
    let mut entry_selector = 0u16;
    while search_range * 2 <= count.max(1) {
        search_range *= 2;
        entry_selector += 1;
    }
    let search_range = search_range * 16;
    let range_shift = count * 16 - search_range.min(count * 16);

    let mut out = Vec::new();
    out.extend_from_slice(&flavor.to_be_bytes());
    out.extend_from_slice(&count.to_be_bytes());
    out.extend_from_slice(&search_range.to_be_bytes());
    out.extend_from_slice(&entry_selector.to_be_bytes());
    out.extend_from_slice(&range_shift.to_be_bytes());
    let mut offset = 12 + tables.len() * 16;
    let mut sorted: Vec<&([u8; 4], Vec<u8>)> = tables.iter().collect();
    sorted.sort_by_key(|(tag, _)| *tag);
    for (tag, data) in &sorted {
        out.extend_from_slice(tag);
        out.extend_from_slice(&0u32.to_be_bytes());
        out.extend_from_slice(&(offset as u32).to_be_bytes());
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        offset += (data.len() + 3) & !3;
    }
    for (_, data) in &sorted {
        out.extend_from_slice(data);
        out.resize((out.len() + 3) & !3, 0);
    }
    out
}

#[cfg(test)]
pub(crate) mod test_font {
    //! Minimal synthetic fonts for tests (no binary fixtures).

    use std::io::Write;

    use super::build_sfnt;

    /// A TrueType font with 1000 units/em mapping each `(char, advance)`.
    pub fn ttf(glyphs: &[(char, u16)]) -> Vec<u8> {
        let units_per_em: u16 = 1000;
        let glyph_count = glyphs.len() as u16 + 1;

        let mut head = vec![0u8; 54];
        head[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        head[12..16].copy_from_slice(&0x5F0F_3CF5u32.to_be_bytes());
        head[18..20].copy_from_slice(&units_per_em.to_be_bytes());

        let mut hhea = vec![0u8; 36];
        hhea[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        hhea[34..36].copy_from_slice(&glyph_count.to_be_bytes());

        let mut maxp = Vec::new();
        maxp.extend_from_slice(&0x0000_5000u32.to_be_bytes());
        maxp.extend_from_slice(&glyph_count.to_be_bytes());

        let mut hmtx = Vec::new();
        hmtx.extend_from_slice(&500u16.to_be_bytes()); // .notdef
        hmtx.extend_from_slice(&0i16.to_be_bytes());
        for (_, advance) in glyphs {
            hmtx.extend_from_slice(&advance.to_be_bytes());
            hmtx.extend_from_slice(&0i16.to_be_bytes());
        }

        // cmap: one format 12 subtable (platform 3, encoding 10), one group per glyph.
        let mut subtable = Vec::new();
        subtable.extend_from_slice(&12u16.to_be_bytes());
        subtable.extend_from_slice(&0u16.to_be_bytes());
        subtable.extend_from_slice(&((16 + 12 * glyphs.len()) as u32).to_be_bytes());
        subtable.extend_from_slice(&0u32.to_be_bytes());
        subtable.extend_from_slice(&(glyphs.len() as u32).to_be_bytes());
        let mut sorted: Vec<(usize, char)> = glyphs
            .iter()
            .enumerate()
            .map(|(i, (c, _))| (i, *c))
            .collect();
        sorted.sort_by_key(|(_, c)| *c);
        for (index, ch) in sorted {
            let code = ch as u32;
            subtable.extend_from_slice(&code.to_be_bytes());
            subtable.extend_from_slice(&code.to_be_bytes());
            subtable.extend_from_slice(&(index as u32 + 1).to_be_bytes());
        }
        let mut cmap = Vec::new();
        cmap.extend_from_slice(&0u16.to_be_bytes());
        cmap.extend_from_slice(&1u16.to_be_bytes());
        cmap.extend_from_slice(&3u16.to_be_bytes());
        cmap.extend_from_slice(&10u16.to_be_bytes());
        cmap.extend_from_slice(&12u32.to_be_bytes());
        cmap.extend_from_slice(&subtable);

        build_sfnt(
            0x0001_0000,
            &[
                (*b"head", head),
                (*b"hhea", hhea),
                (*b"maxp", maxp),
                (*b"hmtx", hmtx),
                (*b"cmap", cmap),
            ],
        )
    }

    /// Wrap an sfnt into WOFF1, zlib-compressing every table.
    pub fn woff1(sfnt: &[u8]) -> Vec<u8> {
        let count = u16::from_be_bytes([sfnt[4], sfnt[5]]) as usize;
        let mut tables = Vec::new();
        for index in 0..count {
            let at = 12 + index * 16;
            let tag = &sfnt[at..at + 4];
            let offset = u32::from_be_bytes(sfnt[at + 8..at + 12].try_into().unwrap()) as usize;
            let length = u32::from_be_bytes(sfnt[at + 12..at + 16].try_into().unwrap()) as usize;
            let mut encoder =
                flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
            let raw = &sfnt[offset..offset + length];
            encoder.write_all(raw).unwrap();
            let compressed = encoder.finish().unwrap();
            // WOFF stores a table uncompressed when compression does not help.
            let stored = if compressed.len() < length {
                compressed
            } else {
                raw.to_vec()
            };
            tables.push((tag.to_vec(), stored, length));
        }
        let mut out = vec![0u8; 44 + 20 * count];
        out[0..4].copy_from_slice(b"wOFF");
        out[4..8].copy_from_slice(&sfnt[0..4]);
        out[12..14].copy_from_slice(&(count as u16).to_be_bytes());
        for (index, (tag, data, original)) in tables.iter().enumerate() {
            let at = 44 + index * 20;
            let offset = out.len();
            out[at..at + 4].copy_from_slice(tag);
            out[at + 4..at + 8].copy_from_slice(&(offset as u32).to_be_bytes());
            out[at + 8..at + 12].copy_from_slice(&(data.len() as u32).to_be_bytes());
            out[at + 12..at + 16].copy_from_slice(&(*original as u32).to_be_bytes());
            out.extend_from_slice(data);
            out.resize((out.len() + 3) & !3, 0);
        }
        let length = out.len() as u32;
        out[8..12].copy_from_slice(&length.to_be_bytes());
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        test_font::ttf(&[('A', 500), ('W', 600), ('…', 1000), ('あ', 1000)])
    }

    #[test]
    fn reads_advances_from_ttf_and_woff1() {
        for bytes in [sample(), test_font::woff1(&sample())] {
            let font = GameFont::from_bytes(&bytes).unwrap();
            assert_eq!(font.advance('A', 26.0), Some(13.0));
            assert_eq!(font.advance('W', 20.0), Some(12.0));
            assert_eq!(font.advance('…', 26.0), Some(26.0));
            assert_eq!(font.advance('z', 26.0), None);
        }
    }

    #[test]
    fn metrics_sum_per_character_and_report_missing_glyphs() {
        let metrics = Metrics::Font(Arc::new(GameFont::from_bytes(&sample()).unwrap()));
        assert_eq!(metrics.text_width("AW…", 26.0), 13.0 + 15.6 + 26.0);
        assert_eq!(metrics.missing_chars("AzA♥"), vec!['z', '♥']);
        // Missing glyphs are estimated like a proportional fallback font.
        assert_eq!(metrics.char_width('z', 20.0), (12.0, false));
        assert_eq!(metrics.char_width('漢', 20.0), (20.0, false));
    }

    #[test]
    fn estimate_follows_the_default_mplus_model() {
        let metrics = Metrics::Estimated;
        assert_eq!(metrics.text_width("Ab", 26.0), 26.0);
        assert_eq!(metrics.text_width("…♥", 26.0), 52.0);
        assert_eq!(metrics.text_width("あ", 26.0), 26.0);
    }

    /// A one-table WOFF1 whose table claims `original` bytes.
    fn woff_with_table(stored: &[u8], original: u32) -> Vec<u8> {
        let mut out = vec![0u8; 44 + 20];
        out[..4].copy_from_slice(b"wOFF");
        out[4..8].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        out[12..14].copy_from_slice(&1u16.to_be_bytes());
        out[44..48].copy_from_slice(b"cmap");
        out[48..52].copy_from_slice(&64u32.to_be_bytes());
        out[52..56].copy_from_slice(&(stored.len() as u32).to_be_bytes());
        out[56..60].copy_from_slice(&original.to_be_bytes());
        out.extend_from_slice(stored);
        out
    }

    #[test]
    fn woff_sizes_are_capped_and_never_inflated_past_the_header() {
        use std::io::Write as _;
        // A table claiming more than the cap is refused before any allocation.
        let huge = woff_with_table(&[0; 16], (MAX_SFNT_BYTES + 1) as u32);
        assert!(matches!(
            woff1_to_sfnt(&huge),
            Err(FontError::Malformed("font too large"))
        ));
        // 8 MB of zeros compress to a few KB: claiming 64 KB must stop there.
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
        encoder.write_all(&vec![0u8; 8 * 1024 * 1024]).unwrap();
        let bomb = encoder.finish().unwrap();
        assert!(bomb.len() < 64 * 1024);
        assert!(matches!(
            woff1_to_sfnt(&woff_with_table(&bomb, 64 * 1024)),
            Err(FontError::Malformed("table length"))
        ));
    }

    #[test]
    fn rejects_woff2_and_garbage() {
        assert!(matches!(
            GameFont::from_bytes(b"wOF2...."),
            Err(FontError::Woff2)
        ));
        assert!(GameFont::from_bytes(b"").is_err());
        assert!(GameFont::from_bytes(b"not a font at all").is_err());
    }
}

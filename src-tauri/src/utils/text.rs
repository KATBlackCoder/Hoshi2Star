/// Escape XML/HTML special characters (`&`, `<`, `>`, `"`).
/// Used for both TMX export (xml) and HTML QA report generation.
pub fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

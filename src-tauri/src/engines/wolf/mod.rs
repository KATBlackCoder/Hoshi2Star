pub(crate) mod dat_parser;
pub(crate) mod encoding;
pub mod extractor;
pub(crate) mod filter;
pub mod injector;
pub(crate) mod integrity;
pub mod layout;
pub mod line_breaks;
pub(crate) mod placeholders;
pub(crate) mod sites;
#[cfg(test)]
mod sites_characterisation;
pub mod sources;
// The only module importing the `unwolf` crate (plan UWF, UWF-D11).
pub mod unpack;
pub(crate) mod v3_format;

pub use layout::fit::WolfTextBox;

//! Game font metrics: moved to [`crate::core::text_fit::font`] (shared by
//! every engine), re-exported at this path for its callers and tests.

pub use crate::core::text_fit::font::*;

#[cfg(test)]
pub(crate) use crate::core::text_fit::font::test_font;

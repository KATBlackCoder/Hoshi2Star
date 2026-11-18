//! The fitting ladder shared by every engine (plan DD, L1-B; ADR-009).
//!
//! Fitting one text always climbs the same steps: skip it, keep it as
//! translated when it already fits, leave it, rewrap it at the base size,
//! shrink it one size at a time down to the minimum, continue it on a new
//! page, and otherwise report it too long. What each step does is the
//! engine's: an adapter implements [`TextBox`] for one text, and [`fit`]
//! drives the steps. Adapters are plain types (static dispatch, no trait
//! objects, ADR-006).

use super::{FitOutcome, Overflow};

/// Why a text is reported too long.
#[derive(Debug, Clone, PartialEq)]
pub enum TooLong<L> {
    /// [`Overflow::Leave`] for an engine that then keeps the translation
    /// ([`TextBox::leave_keeps_text`]).
    LeftAsTranslated,
    /// Nothing fits: the rewrapped layout at the base size, if there is one.
    Unfitted(Option<L>),
}

/// One text of an engine, as fitting sees it.
///
/// A layout ([`TextBox::Layout`]) is the text laid out one way; the verdict
/// ([`TextBox::verdict`]) says whether it fits the box. The adapter builds
/// the final result ([`TextBox::Fitted`]) from a layout and its outcome.
pub trait TextBox {
    /// The text laid out one way (lines, pages…).
    type Layout;
    /// The result of fitting: the text to write and its outcome.
    type Fitted;

    /// Why the text is not fitted at all (`None`: it is fitted).
    fn skip_reason(&self) -> Option<String> {
        None
    }

    /// The text as translated, when "already fits" is judged on it before
    /// any rewrap (`None`: always rewrap).
    fn as_translated(&self) -> Option<Self::Layout> {
        None
    }

    /// Whether `layout` fits the box.
    fn verdict(&self, layout: &Self::Layout) -> bool;

    /// [`Overflow::Leave`] keeps the text as translated (otherwise the text
    /// is still rewrapped, and only shrinking is ruled out).
    fn leave_keeps_text(&self) -> bool {
        true
    }

    /// The text rewrapped at the base size (`None`: no layout).
    fn rewrap(&self) -> Option<Self::Layout>;

    /// The outcome of a rewrapped layout that fits.
    fn rewrapped_outcome(&self, _layout: &Self::Layout) -> FitOutcome {
        FitOutcome::Rewrapped
    }

    /// Whether to try smaller sizes, given the rewrapped layout.
    fn shrinkable(&self, overflow: Overflow, _rewrapped: Option<&Self::Layout>) -> bool {
        overflow == Overflow::Shrink
    }

    /// The size the shrink starts below (`None`: no shrink).
    fn shrink_from(&self) -> Option<u32>;

    /// The text laid out at `size`, behind the engine's size code.
    fn size_code(&self, size: u32) -> Option<Self::Layout>;

    /// The text continued on more pages, with its outcome (`None`: no new
    /// page for this engine, text or setting).
    fn paginate(&self, _overflow: Overflow) -> Option<(Self::Layout, FitOutcome)> {
        None
    }

    /// The result for a layout and its outcome.
    fn finish(&self, layout: Self::Layout, outcome: FitOutcome) -> Self::Fitted;

    /// The result of a skipped text.
    fn skipped(&self, reason: String) -> Self::Fitted;

    /// The result of a text reported too long.
    fn too_long(&self, why: TooLong<Self::Layout>) -> Self::Fitted;
}

/// Fit one text: the ladder of every engine.
pub fn fit<B: TextBox>(text_box: &B, overflow: Overflow, min_font_size: u32) -> B::Fitted {
    if let Some(reason) = text_box.skip_reason() {
        return text_box.skipped(reason);
    }
    if let Some(layout) = text_box.as_translated() {
        if text_box.verdict(&layout) {
            return text_box.finish(layout, FitOutcome::Fits);
        }
    }
    if overflow == Overflow::Leave && text_box.leave_keeps_text() {
        return text_box.too_long(TooLong::LeftAsTranslated);
    }
    let rewrapped = text_box.rewrap();
    if let Some(layout) = rewrapped.as_ref() {
        if text_box.verdict(layout) {
            let outcome = text_box.rewrapped_outcome(layout);
            return text_box.finish(rewrapped.expect("checked above"), outcome);
        }
    }
    if text_box.shrinkable(overflow, rewrapped.as_ref()) {
        if let Some(current) = text_box.shrink_from() {
            for size in (min_font_size..current).rev() {
                if let Some(layout) = text_box.size_code(size) {
                    if text_box.verdict(&layout) {
                        return text_box.finish(layout, FitOutcome::Shrunk { size });
                    }
                }
            }
        }
    }
    if let Some((layout, outcome)) = text_box.paginate(overflow) {
        return text_box.finish(layout, outcome);
    }
    text_box.too_long(TooLong::Unfitted(rewrapped))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A text of `len` units in a box of `width` units at size 10; a size
    /// `n` makes it `len * n / 10` units wide.
    struct Units {
        len: u32,
        width: u32,
        skip: Option<&'static str>,
        keep_as_translated: bool,
        pages: bool,
    }

    impl TextBox for Units {
        type Layout = (u32, u32);
        type Fitted = (FitOutcome, Option<(u32, u32)>);

        fn skip_reason(&self) -> Option<String> {
            self.skip.map(str::to_string)
        }
        fn as_translated(&self) -> Option<(u32, u32)> {
            self.keep_as_translated.then_some((self.len, 10))
        }
        fn verdict(&self, layout: &(u32, u32)) -> bool {
            layout.0 * layout.1 / 10 <= self.width
        }
        fn leave_keeps_text(&self) -> bool {
            self.keep_as_translated
        }
        fn rewrap(&self) -> Option<(u32, u32)> {
            Some((self.len, 10))
        }
        fn shrink_from(&self) -> Option<u32> {
            Some(10)
        }
        fn size_code(&self, size: u32) -> Option<(u32, u32)> {
            Some((self.len, size))
        }
        fn paginate(&self, overflow: Overflow) -> Option<((u32, u32), FitOutcome)> {
            (self.pages && overflow != Overflow::Leave)
                .then_some(((self.len, 10), FitOutcome::NewPage { pages: 2 }))
        }
        fn finish(&self, layout: (u32, u32), outcome: FitOutcome) -> Self::Fitted {
            (outcome, Some(layout))
        }
        fn skipped(&self, reason: String) -> Self::Fitted {
            (FitOutcome::Skipped { reason }, None)
        }
        fn too_long(&self, why: TooLong<(u32, u32)>) -> Self::Fitted {
            match why {
                TooLong::LeftAsTranslated => (
                    FitOutcome::TooLong {
                        reason: "left".into(),
                    },
                    None,
                ),
                TooLong::Unfitted(layout) => (
                    FitOutcome::TooLong {
                        reason: "long".into(),
                    },
                    layout,
                ),
            }
        }
    }

    fn units(len: u32, width: u32) -> Units {
        Units {
            len,
            width,
            skip: None,
            keep_as_translated: true,
            pages: false,
        }
    }

    #[test]
    fn climbs_the_ladder_in_order() {
        // Skipped first, even when it fits.
        let skipped = Units {
            skip: Some("codes"),
            ..units(5, 10)
        };
        assert_eq!(
            fit(&skipped, Overflow::Shrink, 8).0,
            FitOutcome::Skipped {
                reason: "codes".into()
            }
        );
        // Fits as translated.
        assert_eq!(fit(&units(10, 10), Overflow::Shrink, 8).0, FitOutcome::Fits);
        // Without an as-translated check, a fitting rewrap is the outcome.
        let rewrap = Units {
            keep_as_translated: false,
            ..units(10, 10)
        };
        assert_eq!(fit(&rewrap, Overflow::Shrink, 8).0, FitOutcome::Rewrapped);
        // Leave keeps the text when the engine says so.
        assert_eq!(
            fit(&units(12, 10), Overflow::Leave, 8),
            (
                FitOutcome::TooLong {
                    reason: "left".into()
                },
                None
            )
        );
        // Shrinks to the largest size that fits: 13 units are 11 at size 9
        // and 10 at size 8; 12 units are 10 at size 9.
        assert_eq!(
            fit(&units(13, 10), Overflow::Shrink, 8).0,
            FitOutcome::Shrunk { size: 8 }
        );
        assert_eq!(
            fit(&units(12, 10), Overflow::Shrink, 8).0,
            FitOutcome::Shrunk { size: 9 }
        );
        // Not below the minimum: too long, with the rewrapped layout.
        assert_eq!(
            fit(&units(20, 10), Overflow::Shrink, 8),
            (
                FitOutcome::TooLong {
                    reason: "long".into()
                },
                Some((20, 10))
            )
        );
        // New page after a failed shrink, or instead of it.
        let paged = Units {
            pages: true,
            ..units(20, 10)
        };
        assert_eq!(
            fit(&paged, Overflow::Shrink, 8).0,
            FitOutcome::NewPage { pages: 2 }
        );
        assert_eq!(
            fit(&paged, Overflow::NewPage, 8).0,
            FitOutcome::NewPage { pages: 2 }
        );
        // Leave without keeping the text: rewrapped, never shrunk.
        let rewrapped_leave = Units {
            keep_as_translated: false,
            ..units(12, 10)
        };
        assert_eq!(
            fit(&rewrapped_leave, Overflow::Leave, 8),
            (
                FitOutcome::TooLong {
                    reason: "long".into()
                },
                Some((12, 10))
            )
        );
    }
}

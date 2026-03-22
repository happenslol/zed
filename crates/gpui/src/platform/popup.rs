use bitflags::bitflags;

use crate::{Bounds, Pixels, Point};

/// Anchor edge/corner on the anchor rectangle.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum PopupAnchor {
    #[default]
    None,
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    BottomLeft,
    TopRight,
    BottomRight,
}

/// Direction the popup surface extends from the anchor point.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum PopupGravity {
    #[default]
    None,
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    BottomLeft,
    TopRight,
    BottomRight,
}

bitflags! {
    /// How the compositor adjusts popup placement when constrained.
    /// Applied in precedence order: Flip, then Slide, then Resize.
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
    pub struct PopupConstraintAdjustment: u32 {
        const SLIDE_X  = 1;
        const SLIDE_Y  = 2;
        const FLIP_X   = 4;
        const FLIP_Y   = 8;
        const RESIZE_X = 16;
        const RESIZE_Y = 32;
    }
}

/// Options for creating an xdg_popup window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PopupOptions {
    /// Rectangle in the parent surface's coordinate space that the popup
    /// anchors to (position + size, in pixels).
    pub anchor_rect: Bounds<Pixels>,

    /// Which edge/corner of the anchor rectangle to anchor to.
    pub anchor: PopupAnchor,

    /// Direction the popup extends from the anchor point.
    pub gravity: PopupGravity,

    /// How the compositor should adjust placement when the popup would be
    /// constrained (e.g. partially off-screen).
    pub constraint_adjustment: PopupConstraintAdjustment,

    /// Additional offset from the calculated anchor point.
    pub offset: Option<Point<Pixels>>,

    /// If true, the compositor will recalculate popup position when the
    /// parent surface moves or resizes.
    pub reactive: bool,
}

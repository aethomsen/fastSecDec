//! Retained actual source/prefix ownership for a finite contact dispatcher.
//! This receipt alone selects no center and proves no global maximum.
use super::*;
mod branch;
mod cover;
mod origin;
pub use cover::ContactDispatchCover;
pub use origin::ContactDispatchOrigin;
#[cfg(test)]
mod tests;

pub use branch::{ContactBranchProduction, ContactDispatchBranch, ContactTerminalComponents};
mod ascent;
pub use ascent::{DispatchAncestorOrder, DispatchedContactCenter};
mod work;
pub use work::{ContactComponentFrontier, ContactComponentWork, ContactDispatchAdvance};
mod driver;
pub use driver::{ContactAmbientWork, ContactBranchWork, FiniteContactDispatcher};
mod recursive;
pub use recursive::{RecursiveContactBranch, RecursiveContactChildState};

mod recursive_ascent;
pub use recursive_ascent::RecursiveContactCenter;

mod recursive_inventory;
mod recursive_work;
pub use recursive_inventory::{
    RecursiveAscentBranch, RecursiveAscentFrontier, RecursiveBranchAscent,
};
pub use recursive_work::{RecursiveComponentLift, RecursiveLiftWork};
mod composite_center;
mod composite_tree;
mod composite_work;
mod lower_anchor;
pub use composite_center::CompositeContactCenter;
use composite_center::CompositeContext;
pub use composite_work::{CompositeCoverLift, CompositeLiftNext, CompositeLiftWork};

#[cfg(test)]
mod recursive_tests;

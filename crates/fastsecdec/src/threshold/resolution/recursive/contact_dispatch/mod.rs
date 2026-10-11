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

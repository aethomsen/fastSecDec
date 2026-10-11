//! Private arithmetic only. Its caller retains the actual finite-root owners.
use super::{GcadError, Result};
use symbolica::atom::{Atom, AtomCore, Symbol};

pub(super) struct Pullback {
    pub image: Atom,
    pub diagonal: Atom,
}

/// Bounds are expressions in preceding coordinates, independent of this unit.
/// No section, sign, coverage or endpoint authority is issued here.
pub(super) fn pullback(
    lower: Option<&Atom>,
    upper: Option<&Atom>,
    unit: Symbol,
) -> Result<Pullback> {
    let u = Atom::var(unit);
    let one_minus = Atom::one() - &u;
    let (image, diagonal) = match (lower, upper) {
        (Some(a), Some(b)) => {
            let width = b - a;
            (a + &width * &u, width)
        }
        (Some(a), None) => (a + &u / &one_minus, one_minus.pow(-2)),
        (None, Some(b)) => (b - &one_minus / &u, u.pow(-2)),
        (None, None) => {
            return Err(GcadError::Unsupported(
                "a doubly unbounded interval needs a separate map policy".into(),
            ));
        }
    };
    Ok(Pullback { image, diagonal })
}

//! Finite normal coefficients of source-owned polynomial numerators.
//!
//! These are formal coefficients on the relative open face. They do not prove
//! a closed unit, regular density, subtraction strip or executable face map.
mod face;
pub mod normal;
pub mod program;
mod source;
#[cfg(test)]
pub(crate) mod tests;
use super::{Error, Result};
use crate::threshold::maps::regular::{Budget, Limits as AlgebraLimits};
pub use face::{FaceCoefficient, NumeratorFace};
pub use source::{NumeratorLink, SourceNumerator, SourceNumerators};
use std::ops::ControlFlow;
use symbolica::atom::{Atom, AtomCore};

#[derive(Clone, Debug)]
pub struct Limits {
    pub algebra: AlgebraLimits,
    pub max_entries: usize,
    pub max_source_nodes: usize,
    pub max_coefficient_nodes: usize,
    pub max_relative_depth: u32,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            algebra: AlgebraLimits::default(),
            max_entries: 1_000_000,
            max_source_nodes: 160_000,
            max_coefficient_nodes: 1_000_000,
            max_relative_depth: 16,
        }
    }
}
struct Work<'a> {
    limits: &'a Limits,
    budget: Budget,
    callback: &'a mut dyn FnMut() -> ControlFlow<()>,
}
impl Work<'_> {
    fn check(&mut self) -> Result<()> {
        self.budget.check()?;
        if (self.callback)().is_break() {
            return Err(Error::Cancelled);
        }
        Ok(())
    }
    fn entries(&mut self, n: usize) -> Result<()> {
        self.check()?;
        if n > self.limits.max_entries {
            return Err(Error::ResourceIncomplete(
                "numerator retained entries".into(),
            ));
        }
        Ok(())
    }
    fn nodes(&mut self, a: &Atom, limit: usize) -> Result<()> {
        self.check()?;
        let mut count = 0usize;
        let mut failure = None;
        a.visitor(&mut |_| {
            if let Err(e) = self.check() {
                failure = Some(e);
                return false;
            }
            count = match count.checked_add(1) {
                Some(n) if n <= limit => n,
                _ => {
                    failure = Some(Error::ResourceIncomplete("numerator node allowance".into()));
                    return false;
                }
            };
            true
        });
        failure.map_or(Ok(()), Err)
    }
}
fn native(e: impl std::fmt::Display) -> Error {
    Error::Native(e.to_string())
}
fn preflight_error(e: crate::threshold::regularization::meromorphic::Error) -> Error {
    use crate::threshold::regularization::meromorphic::Error as E;
    match e {
        E::Unsupported(s) => Error::Unsupported(s.into()),
        E::Invalid(s) => Error::Invalid(s.into()),
        E::ResourceIncomplete(s) => Error::ResourceIncomplete(s.into()),
        E::Cancelled => Error::Cancelled,
        E::Native(s) => Error::Native(s),
    }
}

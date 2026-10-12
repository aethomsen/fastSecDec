//! Shared bounded native first-interval isolation; only retained owners enter.
use super::{Budget, Error, Rational, Result, witness};
use crate::threshold::maps::regular::{BracketProposal, certificate::ProposalError};
use symgcad::{
    output::RootBound,
    roots::{FiberRoot, refine_once},
};
pub(super) enum Admitted<T> {
    Rational(Rational),
    Root(T),
}
pub(super) fn admit<T>(
    mut root: FiberRoot,
    b: &mut Budget,
    mut proposal: impl FnMut(
        BracketProposal,
        &mut Budget,
    ) -> std::result::Result<T, ProposalError<super::Error>>,
) -> Result<Admitted<T>> {
    let mut margin = Rational::one();
    for _ in 0..b.limits.max_refinements {
        b.check()?;
        b.rational(&margin)?;
        if root.interval.0 == root.interval.1 {
            b.rational(&root.interval.0)?;
            return Ok(Admitted::Rational(root.interval.0));
        }
        // A nonzero selected constant can be separated from zero by native
        // isolation. This supports positive monomial charts without treating
        // the constant as a free integration direction.
        if root.interval.0 <= Rational::zero()
            && root.interval.1 >= Rational::zero()
            && !root.fiber.evaluate(&Rational::zero()).is_zero()
        {
            refine_once(&mut root);
            continue;
        }
        let bracket = BracketProposal {
            lower: root.interval.0.clone(),
            upper: root.interval.1.clone(),
            derivative_margin: margin.clone(),
        };
        match proposal(bracket, b) {
            Ok(section) => {
                return Ok(Admitted::Root(section));
            }
            Err(ProposalError::Miss(_)) => {}
            Err(ProposalError::Failure(error)) => return Err(error),
        }
        refine_once(&mut root);
        margin /= Rational::from(2);
    }
    Err(Error::ResourceIncomplete(
        "constant selected-root bracket/minor allowance".into(),
    ))
}
pub(super) fn prepare(
    source: witness::Source<'_>,
    b: &mut Budget,
) -> Result<(RootBound, FiberRoot, RootBound, FiberRoot)> {
    let axis = source
        .axes()
        .first()
        .ok_or_else(|| Error::Invalid("empty constant endpoint geometry".into()))?;
    let lower = axis
        .lower
        .clone()
        .ok_or_else(|| Error::Unsupported("finite first lower endpoint required".into()))?;
    let upper = axis
        .upper
        .clone()
        .ok_or_else(|| Error::Unsupported("finite first upper endpoint required".into()))?;
    let borrowed = source;
    for bound in [&lower, &upper] {
        let p = witness::polynomial_in(borrowed, bound, b)?;
        if (1..p.nvars()).any(|i| p.degree(i) > 0) {
            return Err(Error::Invalid(
                "first endpoint is not a constant polynomial".into(),
            ));
        }
    }
    let mut lo = witness::selected(borrowed, 0, &lower, &[], b)?;
    let mut hi = witness::selected(borrowed, 0, &upper, &[], b)?;
    let mut ordered = false;
    for _ in 0..b.limits.max_refinements {
        b.check()?;
        if lo.interval.1 < hi.interval.0 {
            ordered = true;
            break;
        }
        if lo.interval.0 >= hi.interval.1 {
            return Err(Error::Invalid(
                "first constant endpoints are not strictly ordered".into(),
            ));
        }
        refine_once(&mut lo);
        refine_once(&mut hi);
    }
    if !ordered {
        return Err(Error::ResourceIncomplete(
            "constant endpoint ordering allowance".into(),
        ));
    }
    Ok((lower, lo, upper, hi))
}

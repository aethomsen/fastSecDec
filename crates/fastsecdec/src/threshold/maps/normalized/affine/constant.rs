use super::{Atom, Budget, CellMap, Error, Rational, Result, Symbol, native, witness};
use crate::threshold::maps::regular::certificate::ProposalError;
use crate::threshold::maps::regular::{BracketProposal, RegularSection, SectionSide};
use std::sync::Arc;
use symgcad::{
    output::RootBound,
    roots::{FiberRoot, refine_once},
};

#[derive(Debug)]
enum Value {
    Rational(Rational),
    Root {
        section: Arc<RegularSection>,
        symbol: Symbol,
    },
}
/// One exact original first-axis endpoint, with selected-root identity retained.
#[derive(Debug)]
pub struct ConstantEndpoint {
    bound: RootBound,
    value: Value,
}
impl ConstantEndpoint {
    pub fn original_bound(&self) -> &RootBound {
        &self.bound
    }
    pub fn expression(&self) -> Atom {
        match &self.value {
            Value::Rational(q) => Atom::num(q.clone()),
            Value::Root { symbol, .. } => Atom::var(*symbol),
        }
    }
    pub fn rational(&self) -> Option<&Rational> {
        match &self.value {
            Value::Rational(q) => Some(q),
            _ => None,
        }
    }
    pub fn symbol(&self) -> Option<Symbol> {
        match &self.value {
            Value::Root { symbol, .. } => Some(*symbol),
            _ => None,
        }
    }
    pub fn section(&self) -> Option<&Arc<RegularSection>> {
        match &self.value {
            Value::Root { section, .. } => Some(section),
            _ => None,
        }
    }
    pub fn enclosure(&self) -> [Rational; 2] {
        match &self.value {
            Value::Rational(q) => [q.clone(), q.clone()],
            Value::Root { section, .. } => [
                section.bracket().lower.clone(),
                section.bracket().upper.clone(),
            ],
        }
    }
}
fn prepare(
    source: &Arc<CellMap>,
    side: SectionSide,
    bound: RootBound,
    mut root: FiberRoot,
    b: &mut Budget,
) -> Result<ConstantEndpoint> {
    if root.interval.0 == root.interval.1 {
        b.rational(&root.interval.0)?;
        return Ok(ConstantEndpoint {
            bound,
            value: Value::Rational(root.interval.0),
        });
    }
    let identity = source
        .decomposition()
        .request()
        .source_identity()
        .map_err(native)?;
    let symbol = Symbol::parse(
        format!(
            "constant_{}_{}_{}",
            identity,
            source.cell_index(),
            if matches!(side, SectionSide::Lower) {
                "lower"
            } else {
                "upper"
            }
        ),
        "fastsecdec_affine",
    )
    .map_err(native)?;
    let mut occupied = super::coordinate_symbols(source.decomposition().request());
    occupied.extend(source.unit_coordinates());
    if occupied.contains(&symbol) || !symbol.is_exportable() {
        return Err(Error::Invalid(
            "constant root slot collides with original source roles".into(),
        ));
    }
    let mut margin = Rational::one();
    for _ in 0..b.limits.max_refinements {
        b.check()?;
        b.rational(&margin)?;
        if root.interval.0 == root.interval.1 {
            b.rational(&root.interval.0)?;
            return Ok(ConstantEndpoint {
                bound,
                value: Value::Rational(root.interval.0),
            });
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
        match RegularSection::prepare_proposal(source.clone(), 0, side, bracket, b) {
            Ok(section) => {
                return Ok(ConstantEndpoint {
                    bound,
                    value: Value::Root { section, symbol },
                });
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
pub(super) fn prepare_pair(
    source: &Arc<CellMap>,
    b: &mut Budget,
) -> Result<(ConstantEndpoint, ConstantEndpoint)> {
    let axis = &source.native_axes()[0];
    let lower = axis
        .lower
        .clone()
        .ok_or_else(|| Error::Unsupported("finite first lower endpoint required".into()))?;
    let upper = axis
        .upper
        .clone()
        .ok_or_else(|| Error::Unsupported("finite first upper endpoint required".into()))?;
    let borrowed = witness::Source::Affine(source);
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
    Ok((
        prepare(source, SectionSide::Lower, lower, lo, b)?,
        prepare(source, SectionSide::Upper, upper, hi, b)?,
    ))
}

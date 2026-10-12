use super::{Atom, Budget, CellMap, Error, Rational, Result, Symbol, native, witness};
use crate::threshold::maps::regular::{RegularSection, SectionSide};
use std::sync::Arc;
use symgcad::{output::RootBound, roots::FiberRoot};

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
    root: FiberRoot,
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
    let value = match super::super::constant_pair::admit(root, b, |bracket, b| {
        RegularSection::prepare_proposal(source.clone(), 0, side, bracket, b)
    })? {
        super::super::constant_pair::Admitted::Rational(q) => Value::Rational(q),
        super::super::constant_pair::Admitted::Root(section) => Value::Root { section, symbol },
    };
    Ok(ConstantEndpoint { bound, value })
}

pub(super) fn prepare_pair(
    source: &Arc<CellMap>,
    b: &mut Budget,
) -> Result<(ConstantEndpoint, ConstantEndpoint)> {
    let (lower, lo, upper, hi) =
        super::super::constant_pair::prepare(witness::Source::Affine(source), b)?;
    Ok((
        prepare(source, SectionSide::Lower, lower, lo, b)?,
        prepare(source, SectionSide::Upper, upper, hi, b)?,
    ))
}

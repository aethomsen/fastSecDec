mod preflight;
use super::{Error, Limits, Result, Work, native};
use crate::{
    parametric::FactorRole,
    threshold::{maps::regular::Budget, projective::PositiveProjectivePreparation},
};
use preflight::preflight;
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::ControlFlow,
    sync::Arc,
};
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    domains::atom::AtomField,
};

/// Every declared factor remains associated with its original source index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NumeratorLink {
    Geometric,
    Inactive { record: usize },
    Active { record: usize, exponent: u64 },
}
#[derive(Debug)]
pub struct SourceNumerator {
    body: Atom,
    degrees: Option<Vec<u32>>,
    exact_zero: bool,
}
impl SourceNumerator {
    pub fn body(&self) -> &Atom {
        &self.body
    }
    /// Genuine preparation coordinate order, before coefficient specialization.
    pub fn degrees(&self) -> Option<&[u32]> {
        self.degrees.as_deref()
    }
    pub fn exact_zero(&self) -> bool {
        self.exact_zero
    }
}
#[derive(Debug)]
pub struct SourceNumerators {
    preparation: Arc<PositiveProjectivePreparation>,
    records: Vec<SourceNumerator>,
    links: Vec<Vec<NumeratorLink>>,
}
impl SourceNumerators {
    pub fn preparation(&self) -> &Arc<PositiveProjectivePreparation> {
        &self.preparation
    }
    pub fn records(&self) -> &[SourceNumerator] {
        &self.records
    }
    pub fn links(&self) -> &[Vec<NumeratorLink>] {
        &self.links
    }
    pub fn prepare(
        preparation: Arc<PositiveProjectivePreparation>,
        limits: &Limits,
        mut progress: impl FnMut() -> ControlFlow<()>,
    ) -> Result<Arc<Self>> {
        let mut work = Work {
            limits,
            budget: Budget::new(limits.algebra.clone()),
            callback: &mut progress,
        };
        work.entries(preparation.terms().len())?;
        let mut records = Vec::new();
        let mut index = BTreeMap::new();
        let mut links = Vec::with_capacity(preparation.terms().len());
        let mut entries = 0usize;
        for term in preparation.terms() {
            work.check()?;
            entries = entries
                .checked_add(term.factors().len())
                .ok_or_else(|| Error::ResourceIncomplete("source link overflow".into()))?;
            work.entries(entries)?;
            let mut row = Vec::with_capacity(term.factors().len());
            for factor in term.factors() {
                if factor.role() != FactorRole::Polynomial {
                    row.push(NumeratorLink::Geometric);
                    continue;
                }
                let exponent = u64::try_from(factor.exponent().as_view()).map_err(native)?;
                if exponent > limits.algebra.max_degree as u64 {
                    return Err(Error::ResourceIncomplete(
                        "numerator source exponent".into(),
                    ));
                }
                let body = factor.polynomial();
                let record = if let Some(&record) = index.get(body) {
                    record
                } else {
                    let record = records.len();
                    work.entries(record.checked_add(1).ok_or_else(|| {
                        Error::ResourceIncomplete("source record overflow".into())
                    })?)?;
                    index.insert(body.clone(), record);
                    records.push(SourceNumerator {
                        body: body.clone(),
                        degrees: None,
                        exact_zero: body.is_zero(),
                    });
                    record
                };
                if exponent != 0 && records[record].degrees.is_none() {
                    records[record].degrees =
                        Some(degrees(body, preparation.coordinates(), &mut work)?);
                }
                row.push(if exponent == 0 {
                    NumeratorLink::Inactive { record }
                } else {
                    NumeratorLink::Active { record, exponent }
                });
            }
            links.push(row);
        }
        work.check()?;
        Ok(Arc::new(Self {
            preparation,
            records,
            links,
        }))
    }
}

fn degrees(body: &Atom, coordinates: &[Symbol], work: &mut Work<'_>) -> Result<Vec<u32>> {
    if coordinates.iter().collect::<BTreeSet<_>>().len() != coordinates.len() {
        return Err(Error::Invalid("duplicate numerator source role".into()));
    }
    work.nodes(body, work.limits.max_source_nodes)?;
    let field = AtomField {
        statistical_zero_test: false,
        ..Default::default()
    };
    let mut degrees = Vec::with_capacity(coordinates.len());
    for coordinate in coordinates {
        preflight(body, *coordinate, work)?;
        if body.is_zero() {
            degrees.push(0);
            continue;
        }
        let x = Atom::var(*coordinate);
        let polynomial =
            body.to_polynomial_in_vars_with_field::<u32>(std::slice::from_ref(&x), &field);
        if polynomial.nterms() > work.limits.algebra.max_terms
            || polynomial.degree(0) as usize > work.limits.algebra.max_degree
        {
            return Err(Error::ResourceIncomplete(
                "native numerator degree/support".into(),
            ));
        }
        for term in &polynomial {
            work.check()?;
            if term.coefficient.contains(x.as_view()) {
                return Err(Error::Unsupported(
                    "hidden source-coordinate dependence in numerator coefficient".into(),
                ));
            }
            work.nodes(term.coefficient, work.limits.max_coefficient_nodes)?;
        }
        degrees.push(polynomial.degree(0));
    }
    Ok(degrees)
}

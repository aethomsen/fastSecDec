//! Complete source inventory, full-epsilon density and common-strip issuance.
use super::projective_family::algebra::pull;
use super::*;
use crate::{
    parametric::{FactorRole, FactorSemantics},
    threshold::gcad::ProjectiveFactorAuthority,
};
use std::collections::BTreeSet;
use symbolica::atom::AtomCore;

fn fresh(occupied: &mut BTreeSet<Symbol>, label: &str) -> Result<Symbol> {
    for i in 0..10_000 {
        let s = Symbol::parse(
            format!("fastsecdec::projective_pair::{label}_{i}"),
            "fastsecdec",
        )
        .map_err(native)?;
        if s.is_exportable() && occupied.insert(s) {
            return Ok(s);
        }
    }
    Err(Error::ResourceIncomplete("fresh pair-family role".into()))
}
fn add_powers(target: &mut [Atom], power: &Atom, orders: &[i64]) {
    for (t, o) in target.iter_mut().zip(orders) {
        *t += power * Atom::num(*o);
    }
}
fn source_sign(
    owner: &PositiveProjectiveQuotient,
    cell: usize,
    term: usize,
    factor: usize,
) -> Result<i8> {
    match owner
        .factor_authorities()
        .get(term)
        .and_then(|r| r.get(factor))
        .ok_or_else(|| invalid("source factor authority ordinal"))?
    {
        ProjectiveFactorAuthority::Split {
            index,
            original_scale,
            quotient_scale,
        } => {
            if *original_scale <= 0 || *quotient_scale <= 0 {
                return Err(invalid("positive original/quotient factor normalization"));
            }
            let s = *owner.native_result().cells[cell]
                .signs
                .get(*index)
                .ok_or_else(|| invalid("native cell split index"))?;
            if !matches!(s, -1 | 1) {
                return Err(unsupported("original cell factor lacks strict sign"));
            }
            Ok(s)
        }
        ProjectiveFactorAuthority::Positive { .. } => Ok(1),
        _ => Err(invalid(
            "inactive/polynomial factor used as a signed source factor",
        )),
    }
}

pub(super) fn admit(
    owner: Arc<PositiveProjectiveQuotient>,
    coordinates: Vec<Symbol>,
    proposals: Vec<CellProposal>,
    limits: Limits,
    settings: OptimizationSettings,
    observer: &mut dyn FnMut(Progress) -> ControlFlow<()>,
) -> Result<ResolvedProjectivePairFamily> {
    if owner.preparation().coordinates().len() != 2
        || coordinates.len() != 2
        || owner.cell_count() == 0
        || coordinates.iter().collect::<BTreeSet<_>>().len() != 2
    {
        return Err(unsupported(
            "complete nonempty two-dimensional positive-projective family required",
        ));
    }
    if proposals.len() != owner.cell_count()
        || proposals.iter().map(|p| p.cell).collect::<BTreeSet<_>>()
            != (0..owner.cell_count()).collect::<BTreeSet<_>>()
    {
        return Err(invalid(
            "every original source cell must occur exactly once",
        ));
    }
    let mut occupied = owner.occupied_symbols().clone();
    occupied.insert(symbolica::transcendental::root_var());
    for s in &coordinates {
        if !s.is_exportable() || !occupied.insert(*s) {
            return Err(invalid("fresh distinct chart roles required"));
        }
    }
    // Root role reuse between separate pieces is allowed; within each piece the
    // inventory is exact and cannot alias source, chart, hook or callback roles.
    for p in &proposals {
        for piece in &p.pieces {
            let mut local = occupied.clone();
            for r in &piece.roots {
                if !r.role.is_exportable() || !local.insert(r.role) {
                    return Err(invalid("fresh distinct normalized root roles required"));
                }
            }
        }
    }
    for p in &proposals {
        for piece in &p.pieces {
            occupied.extend(piece.roots.iter().map(|r| r.role));
        }
    }
    let minimum = proposals
        .iter()
        .try_fold(0usize, |n, p| {
            p.pieces.len().checked_mul(4).and_then(|m| n.checked_add(m))
        })
        .ok_or_else(|| Error::ResourceIncomplete("complete half-cube count overflow".into()))?;
    if minimum > limits.max_charts {
        return Err(Error::ResourceIncomplete(
            "complete half-cube allowance".into(),
        ));
    }
    let primitive = fresh(&mut occupied, "primitive")?;
    let eps = owner.preparation().input().regulator();
    let regulators = vec![eps];
    let mut w = Work {
        limits: &limits,
        budget: crate::threshold::maps::regular::Budget::new(limits.algebra.clone()),
        observer,
    };
    let prepared = owner.preparation();
    let mut inputs = Vec::new();
    let mut prefactors = Vec::new();
    let mut templates = Vec::new();
    for (axis, s) in prepared.coordinates().iter().enumerate() {
        if prepared
            .terms()
            .iter()
            .any(|t| !t.monomial_powers()[axis].is_zero())
        {
            inputs.push((FactorOrigin::Coordinate(axis), Atom::var(*s), Atom::var(*s)));
        }
    }
    for (ti, term) in prepared.terms().iter().enumerate() {
        prefactors.push(
            meromorphic::MeromorphicPrefactor::admit(
                term.prefactor().clone(),
                eps,
                limits.prefactors,
            )
            .map_err(prefactor_error)?,
        );
        let mut numerator = Atom::one();
        let mut indices = Vec::new();
        for power in term.monomial_powers() {
            crate::generation::endpoint_power_with_regulators(power, &regulators)
                .map_err(native)?;
        }
        for (fi, factor) in term.factors().iter().enumerate() {
            if factor.exponent().is_zero() {
                continue;
            }
            if factor.role() == FactorRole::Polynomial {
                let exponent = Rational::try_from(factor.exponent().as_view())
                    .map_err(|_| unsupported("polynomial numerator integer power required"))?;
                if !exponent.is_integer() || exponent < 0 {
                    return Err(unsupported(
                        "polynomial numerator nonnegative integer power required",
                    ));
                }
                w.budget.rational(&exponent)?;
                if exponent > w.limits.prefactors.integer_power {
                    return Err(Error::ResourceIncomplete(
                        "polynomial numerator exponent allowance".into(),
                    ));
                }
                meromorphic::preflight_expression(factor.polynomial(), limits.prefactors)
                    .map_err(prefactor_error)?;
                numerator *= factor.polynomial().pow(Atom::num(exponent));
                indices.push(fi);
            } else {
                crate::generation::endpoint_power_with_regulators(factor.exponent(), &regulators)
                    .map_err(native)?;
                let canonical = projective_family::geometry::rational_source(
                    factor.polynomial(),
                    prepared.coordinates(),
                    &w,
                )?;
                inputs.push((
                    FactorOrigin::Factor {
                        term: ti,
                        factor: fi,
                    },
                    factor.polynomial().clone(),
                    canonical,
                ));
            }
        }
        let roles = (0..4)
            .map(|i| fresh(&mut occupied, &format!("numerator_{ti}_{i}")))
            .collect::<Result<Vec<_>>>()?;
        templates.push(numerator::prepare(
            ti,
            indices,
            numerator,
            prepared.coordinates(),
            eps,
            roles,
            &w,
        )?);
    }
    let mut charts = Vec::new();
    let mut strip = ConvergenceStrip::default();
    for proposal in &proposals {
        w.emit(Progress::Cell(proposal.cell))?;
        let bases = geometry::bases(&owner, &coordinates, proposal, primitive, &settings, &mut w)?;
        for base in bases {
            let mut leaves =
                leaves::build(&owner, &coordinates, base, &inputs, &templates, &mut w)?;
            if charts.len().saturating_add(leaves.len()) > limits.max_charts {
                return Err(Error::ResourceIncomplete(
                    "complete family leaf allowance".into(),
                ));
            }
            for chart in &mut leaves {
                w.emit(Progress::Chart(charts.len()))?;
                let root_roles = chart
                    .sections
                    .iter()
                    .map(|s| s.root_symbol())
                    .collect::<Vec<_>>();
                let root_calls = chart
                    .sections
                    .iter()
                    .zip(&chart.root_inputs)
                    .map(|(s, (tag, c))| s.root_program().call(*tag, c).map_err(native))
                    .collect::<Result<Vec<_>>>()?;
                let measure = chart
                    .receipts
                    .iter()
                    .find(|r| r.origin() == FactorOrigin::Measure)
                    .ok_or_else(|| invalid("measure receipt"))?;
                for (ti, term) in prepared.terms().iter().enumerate() {
                    let numerator = &chart.numerators[ti];
                    let mut powers = measure
                        .powers()
                        .iter()
                        .map(|p| Atom::num(*p))
                        .collect::<Vec<_>>();
                    add_powers(&mut powers, &Atom::one(), numerator.powers());
                    let mut regular =
                        pull(&measure.magnitude(), &root_roles, &root_calls) * numerator.regular();
                    let mut phase = Atom::one();
                    for (axis, power) in term.monomial_powers().iter().enumerate() {
                        if power.is_zero() {
                            continue;
                        }
                        let r = chart
                            .receipts
                            .iter()
                            .find(|r| r.origin() == FactorOrigin::Coordinate(axis))
                            .ok_or_else(|| invalid("active coordinate receipt"))?;
                        if r.sign() != 1 {
                            return Err(invalid("positive source coordinate sign"));
                        }
                        add_powers(&mut powers, power, r.powers());
                        regular *= pull(&r.magnitude(), &root_roles, &root_calls).pow(power);
                    }
                    for (fi, factor) in term.factors().iter().enumerate() {
                        if factor.exponent().is_zero() || factor.role() == FactorRole::Polynomial {
                            continue;
                        }
                        let r = chart
                            .receipts
                            .iter()
                            .find(|r| {
                                r.origin()
                                    == FactorOrigin::Factor {
                                        term: ti,
                                        factor: fi,
                                    }
                            })
                            .ok_or_else(|| invalid("active factor receipt"))?;
                        let expected = source_sign(&owner, chart.cell, ti, fi)?;
                        if r.sign() != expected {
                            return Err(invalid(
                                "closed factor sign differs from original native cell",
                            ));
                        }
                        if expected < 0 {
                            if factor.semantics() != FactorSemantics::Causal {
                                return Err(unsupported(
                                    "negative factor requires original causal branch semantics",
                                ));
                            }
                            phase *= crate::threshold::phase::causal_negative_phase(
                                factor.exponent(),
                                &regulators,
                            )
                            .map_err(native)?;
                        }
                        add_powers(&mut powers, factor.exponent(), r.powers());
                        regular *=
                            pull(&r.magnitude(), &root_roles, &root_calls).pow(factor.exponent());
                    }
                    for power in &powers {
                        strip.admit(power, eps).map_err(native)?;
                    }
                    chart.terms.push(NormalizedTerm {
                        term: ti,
                        prefactor: term.prefactor() * phase,
                        regular,
                        powers,
                    });
                }
            }
            charts.extend(leaves);
        }
    }
    let witness = meromorphic::MeromorphicWitness::construct(
        prefactors,
        strip.lower(),
        strip.upper(),
        limits.prefactors,
        |_| {
            if w.check().is_ok() {
                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(())
            }
        },
    )
    .map_err(prefactor_error)?;
    w.check()?;
    let roots = Scope::merged(charts.iter().map(|c| &c.roots)).map_err(native)?;
    Ok(ResolvedProjectivePairFamily {
        owner,
        coordinates,
        regulators,
        charts,
        strip,
        witness,
        roots,
    })
}

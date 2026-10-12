//! Genuine root-call bindings for the existing symbolic continuation/compiler.
//! No independent evaluator or epsilon-zero specialization is introduced here.
use super::*;
use crate::{
    kernel::algebraic::Scope,
    threshold::maps::{endpoint_transport::clearing::pull, normalized::algebra::Ring},
};
use std::collections::BTreeSet;
use symbolica::symbol;
pub struct RootBindings {
    symbols: Vec<Symbol>,
    calls: Vec<Atom>,
    scope: Scope,
}
impl RootBindings {
    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }
    pub fn calls(&self) -> &[Atom] {
        &self.calls
    }
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    pub fn substitute(&self, body: &Atom) -> Atom {
        pull(body, &self.symbols, &self.calls)
    }
}
pub(super) fn prepare(
    chart: &IssuedEndpointChart,
    settings: OptimizationSettings,
    budget: &Budget,
) -> Result<RootBindings> {
    let roots = chart
        .transport()
        .root_relations()
        .iter()
        .map(|r| r.section().root_symbol())
        .collect::<Vec<_>>();
    let ring = Ring::new(chart.target_units().iter().chain(&roots).copied().collect())?;
    let mut available = chart
        .target_units()
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut occupied = available.clone();
    occupied.extend(&roots);
    let source = chart.parent_prefix().source().decomposition().preparation();
    for term in source.terms() {
        occupied.extend(term.prefactor().get_all_symbols(true));
        for power in term.monomial_powers() {
            occupied.extend(power.get_all_symbols(true));
        }
        for f in term.factors() {
            occupied.extend(f.polynomial().get_all_symbols(true));
            occupied.extend(f.exponent().get_all_symbols(true));
        }
    }
    for relation in chart.transport().root_relations() {
        occupied.extend(&relation.section().chart().parameters);
        occupied.extend(relation.equation().get_all_symbols(true));
    }
    let mut calls = Vec::new();
    let mut scope = Scope::default();
    for (i, r) in chart.transport().root_relations().iter().enumerate() {
        budget.check()?;
        let section = r.section();
        let parameters = chart.section_parameter_images(section)?;
        occupied.extend(&section.chart().parameters);
        for p in &parameters {
            if !p
                .get_all_symbols(true)
                .iter()
                .all(|s| available.contains(s))
            {
                return Err(Error::Invalid(
                    "closed selected-root prefix is not triangular".into(),
                ));
            }
            ring.polynomial(p, budget)?;
        }
        // The sealed polynomial unit map carries the original closed-box
        // containment and nonzero-minor proof by continuous composition.
        let program = section.root_program(settings.clone())?;
        let mut tag = None;
        for n in 0..1024usize {
            budget.check()?;
            let candidate = symbol!(format!("fastsecdec::closed_scalar_{n}::root_{i}"));
            if candidate.is_exportable() && !occupied.contains(&candidate) {
                tag = Some(candidate);
                break;
            }
        }
        let tag =
            tag.ok_or_else(|| Error::ResourceIncomplete("closed root tag allowance".into()))?;
        occupied.insert(tag);
        let call = program.call(tag, section.coefficients())?;
        let call = pull(
            &pull(&call, &section.chart().parameters, &parameters),
            &roots[..i],
            &calls,
        );
        scope.insert(tag, program)?;
        calls.push(call);
        available.insert(section.root_symbol());
    }
    Ok(RootBindings {
        symbols: roots,
        calls,
        scope,
    })
}

//! Native scaling-degree admission without expanding regular numerator support.

use symbolica::{
    atom::{Atom, AtomCore, Symbol, SymbolAttribute, SymbolBuilder},
    domains::atom::AtomField,
    id::{Pattern, Replacement},
    wrap_symbol,
};

/// A sole scaling power proves homogeneity, not that its coefficient is nonzero.
/// Return `None` when native recognition is inconclusive so the existing sparse
/// support admission can resolve cancellations or reject nonpolynomial input.
pub(crate) fn degree(expression: &Atom, parameters: &[Symbol]) -> Option<u32> {
    let atoms = parameters.iter().map(|s| Atom::var(*s)).collect::<Vec<_>>();
    let indeterminates = expression.is_polynomial(true, false)?;
    if !indeterminates.iter().all(|indeterminate| {
        atoms.iter().any(|p| *indeterminate == p.as_view())
            || atoms.iter().all(|p| !indeterminate.contains(p.as_view()))
    }) {
        return None;
    }

    // A reused native formal symbol needs no per-job global names or callbacks.
    // An occupied or incompatible name simply selects the existing fallback.
    let scale = SymbolBuilder::new(wrap_symbol!("fastsecdec::parametric::scale"))
        .with_attributes(&[] as &[SymbolAttribute])
        .build()
        .ok()?;
    if !scale.is_exportable() || expression.contains_symbol(scale) || parameters.contains(&scale) {
        return None;
    }
    let scale = Atom::var(scale);
    let replacements = atoms
        .iter()
        .map(|p| Replacement::new(Pattern::Literal(p.clone()), Pattern::Literal(p * &scale)))
        .collect::<Vec<_>>();
    let scaled = expression.replace_multiple(&replacements);
    let field = AtomField {
        statistical_zero_test: false,
        cancel_check_on_division: false,
        custom_normalization: None,
    };
    let polynomial = scaled.to_polynomial_in_vars_with_field::<u32>([scale.clone()], &field);
    if polynomial.nterms() != 1 {
        return None;
    }
    let term = (&polynomial).into_iter().next()?;
    if term.coefficient.contains(scale.as_view()) {
        return None;
    }
    Some(term.exponents[0])
}

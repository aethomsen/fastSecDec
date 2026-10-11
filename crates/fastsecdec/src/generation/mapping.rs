use super::{CoordinateMap, GenerationError};
use crate::parametric::ParametricIntegrand;
use fastsecdec_sectors::SectorMap;
use std::collections::BTreeMap;
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    domains::{atom::AtomField, integer::Integer},
    id::{Pattern, Replacement},
};

pub(super) mod contour;
mod regular;
pub(super) use contour::{PreparedTerm, declared_factors};

#[cfg(test)]
pub(super) mod profile;

// Attribution is confined to the unit-test build. The ordinary production
// expression is unchanged and has no timers or profile state.
#[cfg(test)]
macro_rules! measured {
    ($stage:ident, $expression:expr) => {
        profile::measure(profile::Stage::$stage, || $expression)
    };
}
#[cfg(not(test))]
macro_rules! measured {
    ($stage:ident, $expression:expr) => {
        $expression
    };
}

pub(super) struct MappedTerm {
    pub powers: Vec<Atom>,
    pub prefactor: Atom,
    pub regular: Atom,
}
pub(super) type MappedOutput = (
    Vec<MappedTerm>,
    Option<crate::contour::ContourMetadata>,
    super::program::ProgramData,
);

/// Extract monomials using exact Newton support, leaving nonnegative powers in
/// every residual polynomial. Numerator factors share the map but do not refine
/// the fan. Symbolica owns substitution, sparse polynomial collection and
/// coefficient arithmetic; mapped factors stay factored unless a nonzero
/// monomial valuation must actually be removed.
pub(super) fn map_terms(
    input: &ParametricIntegrand,
    map: &SectorMap,
    coordinates: &CoordinateMap,
    source_supports: &mut super::support::SupportCache,
) -> Result<Vec<MappedTerm>, GenerationError> {
    map_terms_with_contour(
        input,
        map,
        coordinates,
        source_supports,
        crate::kernel::indexed::ProgramRecipe::UndeformedV1,
        crate::contour::ContourJacobian::Symbolic,
        true,
    )
    .map(|(terms, _, _)| terms)
}

pub(super) fn map_terms_with_contour(
    input: &ParametricIntegrand,
    map: &SectorMap,
    coordinates: &CoordinateMap,
    source_supports: &mut super::support::SupportCache,
    recipe: crate::kernel::indexed::ProgramRecipe,
    jacobian: crate::contour::ContourJacobian,
    combine: bool,
) -> Result<MappedOutput, GenerationError> {
    let terms = prepare_terms(
        input,
        map,
        coordinates,
        source_supports,
        recipe.is_contour(),
    )?;
    apply_prepared(
        coordinates.target_parameters(),
        terms,
        recipe,
        jacobian,
        combine,
    )
}

/// Perform the expensive native substitution and monomial extraction once.
/// All declared zero-exponent branch factors are retained when requested, so
/// later recipe selection sees the original complete residual F/U set.
pub(super) fn prepare_terms(
    input: &ParametricIntegrand,
    map: &SectorMap,
    coordinates: &CoordinateMap,
    source_supports: &mut super::support::SupportCache,
    retain_declarations: bool,
) -> Result<Vec<PreparedTerm>, GenerationError> {
    #[cfg(test)]
    profile::begin_chart();
    let parameters = coordinates.target_parameters();
    let variables = parameters.iter().map(|p| Atom::var(*p)).collect::<Vec<_>>();
    let images = coordinates.images();
    let nonnegative_map = map
        .exponent_matrix
        .iter()
        .flatten()
        .all(|power| power >= &0);
    let mut prepared = Vec::with_capacity(input.terms().len());
    for term in input.terms() {
        #[cfg(test)]
        profile::begin_term();
        let mut powers = coordinates.measure_powers.clone();
        for (row, power) in map.exponent_matrix.iter().zip(term.monomial_powers()) {
            add_scaled_powers(&mut powers, power, row)?;
        }
        let prefactor = term.prefactor() * &coordinates.measure_factor;
        let mut residuals = Vec::new();
        for factor in term.factors() {
            #[cfg(test)]
            profile::begin_factor(factor);
            if factor.exponent().is_zero() && !retain_declarations {
                continue;
            }
            let mapped = measured!(
                Substitution,
                factor
                    .polynomial()
                    .replace_multiple(input.parameters().iter().zip(images).map(
                        |(source, target)| {
                            Replacement::new(
                                Pattern::Literal(Atom::var(*source)),
                                Pattern::Literal(target.clone()),
                            )
                        },
                    ))
            );
            // For a polynomial map, nonvanishing coordinate faces exclude a
            // common coordinate monomial without enumerating dense support.
            // This preserves both (1+x)^10000 and (x+y)^10000. Singular factors
            // still receive exact polynomial residual/domain validation below.
            let zero_valuation = measured!(
                CoordinateFaces,
                nonnegative_map
                    && variables.iter().all(|variable| {
                        !mapped
                            .replace(Pattern::Literal(variable.clone()))
                            .with(Atom::Zero)
                            .is_zero()
                    })
            );
            let (minima, residual) = if zero_valuation {
                (vec![Integer::from(0); parameters.len()], mapped)
            } else if let Some(candidate) = (nonnegative_map
                && factor.role() == crate::parametric::FactorRole::Polynomial)
                .then(|| {
                    measured!(
                        RegularMonomial,
                        regular::common_monomial(&mapped, &variables)
                    )
                })
                .flatten()
            {
                candidate
            } else {
                let support = measured!(SupportExtraction, source_supports.get(factor))?;
                #[cfg(test)]
                profile::support_size(support.exponents().len());
                let (transformed, minima) = measured!(SupportTransformation, {
                    let transformed = support
                        .exponents()
                        .iter()
                        .map(|row| {
                            (0..parameters.len())
                                .map(|j| {
                                    row.iter()
                                        .zip(&map.exponent_matrix)
                                        .fold(Integer::from(0), |sum, (a, m)| sum + a * &m[j])
                                })
                                .collect::<Vec<_>>()
                        })
                        .collect::<Vec<_>>();
                    let minima = (0..parameters.len())
                        .map(|j| transformed.iter().map(|row| &row[j]).min().unwrap().clone())
                        .collect::<Vec<_>>();
                    (transformed, minima)
                });
                let residual = if minima.iter().all(|power| power == &0) {
                    mapped
                } else {
                    // Signed native polynomial exponents support orthant
                    // infinity charts. Bound conversion before invoking the
                    // fixed-width native collector and monomial shift.
                    let signed = |value: &Integer| {
                        value
                            .to_i64()
                            .and_then(|v| i32::try_from(v).ok())
                            .ok_or(GenerationError::ResourceLimit("mapped polynomial exponent"))
                    };
                    for row in &transformed {
                        for (power, minimum) in row.iter().zip(&minima) {
                            signed(power)?;
                            signed(&(power - minimum))?;
                        }
                    }
                    let shifts = minima
                        .iter()
                        .map(|value| signed(&-value))
                        .collect::<Result<Vec<_>, _>>()?;
                    if let Some(residual) = factored_residual(&mapped, &variables, &shifts) {
                        residual
                    } else {
                        let polynomial = measured!(
                            SparseFallback,
                            mapped
                                .to_polynomial_in_vars_with_field::<i32>(
                                    &variables,
                                    &AtomField {
                                        statistical_zero_test: false,
                                        ..AtomField::new()
                                    },
                                )
                                .mul_exp(&shifts)
                        );
                        if polynomial
                            .exponents_iter()
                            .flatten()
                            .any(|power| *power < 0)
                        {
                            return Err(GenerationError::Invariant(
                                "negative exponent after monomial extraction".into(),
                            ));
                        }
                        polynomial.flatten(false)
                    }
                };
                (minima, residual)
            };
            if super::domain::is_singular(factor) {
                measured!(
                    ResidualCertification,
                    super::domain::check_residual(&residual, parameters)
                )?;
            }
            add_scaled_powers(&mut powers, factor.exponent(), &minima)?;
            residuals.push((residual, factor.exponent().clone(), factor.semantics()));
        }
        let powers = powers.into_iter().map(|p| p.expand()).collect::<Vec<_>>();
        prepared.push(PreparedTerm {
            powers,
            prefactor,
            residuals,
        });
    }
    Ok(prepared)
}

pub(super) fn apply_prepared(
    parameters: &[Symbol],
    terms: Vec<PreparedTerm>,
    recipe: crate::kernel::indexed::ProgramRecipe,
    jacobian: crate::contour::ContourJacobian,
    combine: bool,
) -> Result<MappedOutput, GenerationError> {
    let mut program = super::program::ProgramData::default();
    let mut witness = None;
    let mut combined = BTreeMap::<Vec<Atom>, BTreeMap<Atom, Atom>>::new();
    let mut separate = Vec::new();
    let contour_metadata = if recipe.is_contour() {
        let (terms, metadata, source_witness) =
            contour::deform_with(parameters, terms, recipe, |parameters, f, positive| {
                let (map, owner) =
                    contour::program::build(recipe, jacobian, parameters, f, positive)?;
                program = owner;
                Ok(map)
            })?;
        witness = source_witness;
        for term in terms {
            retain_term(term, combine, &mut combined, &mut separate);
        }
        metadata
    } else {
        for term in terms {
            let regular = term
                .residuals
                .into_iter()
                .filter(|(_, exponent, _)| !exponent.is_zero())
                .map(|(residual, exponent, _)| residual.pow(exponent))
                .product();
            retain_term(
                MappedTerm {
                    powers: term.powers,
                    prefactor: term.prefactor,
                    regular,
                },
                combine,
                &mut combined,
                &mut separate,
            );
        }
        None
    };
    if let Some(witness) = witness {
        program.source_witnesses.push((0, witness));
    }
    if !combine {
        return Ok((separate, contour_metadata, program));
    }
    let terms = combined
        .into_iter()
        .filter_map(|(powers, mut prefactors)| {
            // Preserve cancellation between terms with the same endpoint powers.
            // Only detach a genuinely common, parameter-independent factor.
            let (prefactor, regular) = if prefactors.len() == 1 {
                prefactors.pop_first().unwrap()
            } else {
                (
                    Atom::one(),
                    prefactors.into_iter().map(|(p, r)| p * r).sum(),
                )
            };
            (!regular.is_zero() && !prefactor.is_zero()).then_some(MappedTerm {
                powers,
                prefactor,
                regular,
            })
        })
        .collect();
    Ok((terms, contour_metadata, program))
}

fn retain_term(
    term: MappedTerm,
    combine: bool,
    combined: &mut BTreeMap<Vec<Atom>, BTreeMap<Atom, Atom>>,
    separate: &mut Vec<MappedTerm>,
) {
    if combine {
        *combined
            .entry(term.powers)
            .or_default()
            .entry(term.prefactor)
            .or_insert(Atom::Zero) += term.regular;
    } else if !term.regular.is_zero() && !term.prefactor.is_zero() {
        // Native jets require a regulator-analytic body, with meromorphic
        // source prefactors kept outside the body of each term.
        separate.push(term);
    }
}

/// Native collection exposes chart monomials inside nested sums and integer
/// powers without materializing their polynomial support again. The exact
/// valuation and exponent bounds have already been checked by the caller.
fn factored_residual(mapped: &Atom, variables: &[Atom], shifts: &[i32]) -> Option<Atom> {
    #[cfg(test)]
    if SPARSE_ONLY.with(std::cell::Cell::get) {
        return None;
    }
    let residual = measured!(FactorCollection, mapped.collect_factors())
        * variables
            .iter()
            .zip(shifts)
            .map(|(variable, shift)| variable.pow(Atom::num(*shift)))
            .product::<Atom>();
    measured!(
        PolynomialRecognition,
        residual
            .is_polynomial(true, false)
            .is_some_and(|indeterminates| {
                indeterminates.iter().all(|indeterminate| {
                    variables
                        .iter()
                        .any(|variable| *indeterminate == variable.as_view())
                        || variables
                            .iter()
                            .all(|variable| !indeterminate.contains(variable.as_view()))
                })
            })
            .then_some(residual)
    )
}

#[cfg(test)]
std::thread_local! {
    static SPARSE_ONLY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
pub(super) fn profile_sparse_only(enabled: bool) {
    SPARSE_ONLY.with(|value| value.set(enabled));
}

#[cfg(test)]
mod tests;

pub(super) fn coordinates(
    input: &ParametricIntegrand,
    map: &SectorMap,
    parameters: &[Symbol],
) -> CoordinateMap {
    coordinates_from_parts(input.parameters(), input.domain(), map, parameters)
}

pub(crate) fn coordinates_from_parts(
    source: &[Symbol],
    domain: fastsecdec_sectors::ParametricDomain,
    map: &SectorMap,
    parameters: &[Symbol],
) -> CoordinateMap {
    let variables = parameters.iter().map(|p| Atom::var(*p)).collect::<Vec<_>>();
    let images = map
        .exponent_matrix
        .iter()
        .map(|row| {
            row.iter()
                .zip(&variables)
                .map(|(power, variable)| variable.pow(Atom::num(power.clone())))
                .product()
        })
        .collect();
    let measure_factor = Atom::num(map.determinant.clone());
    let measure_powers = map
        .jacobian_powers
        .iter()
        .cloned()
        .map(Atom::num)
        .collect::<Vec<_>>();
    let measure_jacobian = &measure_factor
        * measure_powers
            .iter()
            .zip(&variables)
            .map(|(power, variable)| variable.pow(power))
            .product::<Atom>();
    CoordinateMap {
        source_parameters: source.to_vec(),
        target_parameters: parameters.to_vec(),
        images,
        measure_jacobian,
        measure_factor,
        measure_powers,
        source_domain: domain,
        projective_fixed_parameter: map.fixed_parameter,
    }
}

/// Keep generated coordinates disjoint from every source symbol, including
/// coordinates absent from the density. Shared by ordinary and stepped callers.
pub(super) fn target_parameters(input: &ParametricIntegrand, dimension: usize) -> Vec<Symbol> {
    let mut source_symbols = input.density().get_all_symbols(true);
    source_symbols.extend(input.parameters().iter().copied());
    source_symbols.insert(input.regulator());
    for namespace in 0usize.. {
        let candidates = (0..dimension)
            .map(|axis| symbolica::symbol!(format!("fastsecdec::sector_{namespace}::t{axis}")))
            .collect::<Vec<_>>();
        if candidates
            .iter()
            .all(|symbol| !source_symbols.contains(symbol))
        {
            return candidates;
        }
    }
    unreachable!("finite source symbols cannot exhaust namespaces")
}

/// Exact exponent transport shared by ordinary and source-bound endpoint maps.
/// This arithmetic grants no residual unit or endpoint authority.
pub(crate) fn add_scaled_powers(
    powers: &mut [Atom],
    exponent: &Atom,
    orders: &[Integer],
) -> Result<(), GenerationError> {
    if powers.len() != orders.len() {
        return Err(GenerationError::Invariant(
            "exponent transport dimension differs".into(),
        ));
    }
    for (power, order) in powers.iter_mut().zip(orders) {
        *power += exponent * Atom::num(order.clone());
    }
    Ok(())
}

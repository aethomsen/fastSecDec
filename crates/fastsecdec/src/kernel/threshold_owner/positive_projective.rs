//! Compact projection of a complete issued positive-projective family. Original
//! homogeneous and quotient evidence stay generation-side; no reduced request
//! is constructed and no saved arithmetic descriptor issues a new atlas.
use super::algebraic::{canonical, digest, push};
use super::resolved;
use super::*;
use crate::threshold::{
    continued::SourceAuthority,
    records::{StagedVector, VectorKind},
    regularization::projective_pair_family::{ContinuedProjectivePairFamily, ResolvedPairChart},
};
use std::{collections::BTreeSet, sync::Arc};
use symbolica::{
    atom::AtomCore,
    id::{Pattern, Replacement},
};

impl ThresholdMetadata {
    pub(super) fn from_positive_projective_family(
        bound: &ContinuedProjectivePairFamily<'_>,
        staged: &[StagedVector],
        orders: Vec<i32>,
    ) -> Result<Self, KernelError> {
        let certificate = bound.certificate();
        let owner = certificate.owner();
        let preparation = owner.preparation();
        let patch = source_patch::SourcePatch::from_positive_projective(owner)?;
        let coordinates = certificate.coordinates();
        if !certificate.runtime_parameters().is_empty()
            || coordinates.len() != patch.coordinates.len()
            || staged.len() != certificate.charts().len()
            || staged.len() != bound.charts().len()
            || staged.iter().any(|r| !r.algebraic)
        {
            return Err(invalid(
                "positive-projective family/source record association",
            ));
        }
        let inventory = certificate
            .charts()
            .iter()
            .map(|c| c.cell_index())
            .collect::<BTreeSet<_>>();
        if inventory != (0..owner.cell_count()).collect::<BTreeSet<_>>() {
            return Err(invalid("positive-projective atlas omits an original cell"));
        }
        let source = m::Digest(
            SourceAuthority::PositiveProjective(owner)
                .source_identity()
                .map_err(invalid)?,
        );
        let original_raw = digest(
            "threshold-native-solve-evidence-v1",
            owner.original_result().as_ref(),
        )?;
        let raw = digest("threshold-native-solve-evidence-v1", owner.native_result())?;
        let problem = digest(
            "threshold-native-problem-v1",
            &owner.native_result().problem,
        )?;
        let anchor_math = serde_json::json!({
            "source": source, "anchor":preparation.anchor_index(),
            "images":preparation.images().iter().map(Atom::to_canonical_string).collect::<Vec<_>>(),
            "normalized_images":preparation.normalized_images().iter().map(Atom::to_canonical_string).collect::<Vec<_>>(),
            "simplex_jacobian":preparation.simplex_jacobian().to_canonical_string(),
            "density_measure":preparation.measure_jacobian().to_canonical_string(),
            "factor_origins":format!("{:?}",preparation.factor_origins()),
            "factor_degrees":preparation.factor_degrees(),
        });
        let anchor = digest(
            "threshold-positive-projective-anchor-v1",
            &serde_json::json!({
                "source": source, "original_evidence": original_raw, "quotient_evidence": raw,
                "anchor":preparation.anchor_index(),
                "images":preparation.images().iter().map(Atom::to_canonical_string).collect::<Vec<_>>(),
                "normalized_images":preparation.normalized_images().iter().map(Atom::to_canonical_string).collect::<Vec<_>>(),
                "simplex_jacobian":preparation.simplex_jacobian().to_canonical_string(),
                "density_measure":preparation.measure_jacobian().to_canonical_string(),
                "factor_origins":format!("{:?}",preparation.factor_origins()),
                "factor_degrees":preparation.factor_degrees(),
            }),
        )?;
        let coverage = digest(
            "threshold-positive-projective-complete-atlas-v1",
            &(
                &anchor,
                &inventory,
                certificate
                    .charts()
                    .iter()
                    .map(|c| (c.cell_index(), c.endpoint_path()))
                    .collect::<Vec<_>>(),
            ),
        )?;
        let mut symbols = preparation.input().parameters().to_vec();
        let original = (0..symbols.len())
            .map(m::NativeSymbolId)
            .collect::<Vec<_>>();
        let prepared = patch
            .coordinates
            .iter()
            .map(|s| role(&symbols, *s))
            .collect::<Result<Vec<_>, _>>()?;
        let regulator = m::NativeSymbolId(symbols.len());
        symbols.push(preparation.input().regulator());
        let units = coordinates
            .iter()
            .map(|s| {
                let id = m::NativeSymbolId(symbols.len());
                symbols.push(*s);
                id
            })
            .collect::<Vec<_>>();
        let mut renames = coordinates
            .iter()
            .enumerate()
            .map(|(i, s)| {
                Symbol::parse(format!("fastsecdec::threshold_semantic::unit_{i}"), "")
                    .map(|target| (*s, target))
                    .map_err(invalid)
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        // Native transport tags depend on optimized IR; mathematical helper
        // identities retain only the selected source/branch/numeric recipe.
        // Keep transport names out of the version-eight semantic projection.
        let mut semantic_helpers = BTreeSet::new();
        let mut root_roles = BTreeMap::new();
        let mut template_roles = BTreeMap::new();
        let mut function_roles = BTreeMap::new();
        for (i, chart) in certificate.charts().iter().enumerate() {
            for (tag, helper) in bound.roots(i).owners() {
                let identity = helper.semantic_identity();
                semantic_helpers.insert(identity.to_owned());
                for (role, name) in [
                    (tag, format!("root_owner_{identity}")),
                    (
                        helper.transport_symbol(),
                        format!("root_contract_{identity}"),
                    ),
                ] {
                    renames.insert(role, semantic_role(&name)?);
                }
            }
            for section in chart.sections() {
                let next = root_roles.len();
                let index = *root_roles.entry(section.root_symbol()).or_insert(next);
                renames.insert(
                    section.root_symbol(),
                    semantic_role(&format!("root_role_{index}"))?,
                );
            }
            for numerator in chart.numerator_transport() {
                for role in numerator.template_roles() {
                    let next = template_roles.len();
                    let index = *template_roles.entry(*role).or_insert(next);
                    renames.insert(*role, semantic_role(&format!("numerator_formal_{index}"))?);
                }
            }
            for definition in bound.definitions(i) {
                if definition.head != Symbol::DERIVATIVE {
                    let next = function_roles.len();
                    let index = *function_roles.entry(definition.head).or_insert(next);
                    renames.insert(
                        definition.head,
                        semantic_role(&format!("numerator_function_{index}"))?,
                    );
                }
            }
        }
        let canonicalize = |a: &Atom| canonical(a, &renames);
        let policy = resolved::continuation_policy(bound.options());
        let actual_options = digest("threshold-resolved-continuation-policy-v1", &policy)?;
        let common = resolved::common_domain(
            certificate.convergence_strip(),
            certificate.prefactor_witness(),
        )?;
        let mut atoms = Vec::new();
        let placeholder = m::Digest("0".repeat(64));
        let mut patch = m::Patch {
            id: m::PatchId(0),
            source: m::SourceOrigin::OriginalInput,
            coverage: coverage.clone(),
            map: m::MapDescriptor::RationalV1 {
                geometry: m::MapGeometry {
                    expressions: placeholder.clone(),
                    coordinates: prepared,
                    images: patch
                        .images
                        .into_iter()
                        .map(|a| push(&mut atoms, a))
                        .collect(),
                    positive_measure: push(&mut atoms, patch.positive_measure),
                    orientation: m::Orientation::Forward,
                    certificate: anchor.clone(),
                },
            },
        };
        let native_order = owner
            .native_result()
            .order
            .iter()
            .map(|name| {
                owner
                    .aliases()
                    .iter()
                    .find(|a| &a.name == name)
                    .map(|a| a.symbol)
                    .ok_or_else(|| invalid("positive-projective native coordinate alias"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut cells = BTreeMap::new();
        let mut charts = Vec::new();
        let mut contributions = Vec::new();
        let mut paths = BTreeSet::new();
        let mut function_ids = Vec::new();
        let mut mathematical_charts = Vec::new();
        for (i, (chart, record)) in certificate.charts().iter().zip(staged).enumerate() {
            let raw_index = chart.cell_index();
            if record.chart != i
                || chart.images().len() != preparation.coordinates().len()
                || chart.coordinate_order() != native_order
                || chart.root_inputs().len() != chart.sections().len()
                || chart.source_axes() != owner.native_result().cells[raw_index].axes.as_slice()
                || !paths.insert((raw_index, chart.endpoint_path()))
            {
                return Err(invalid(
                    "positive-projective chart order/source/path association",
                ));
            }
            let cell = m::Cell {
                id: m::CellId(raw_index),
                patch: m::PatchId(0),
                coordinate_order: chart
                    .coordinate_order()
                    .iter()
                    .map(|s| role(&symbols, *s))
                    .collect::<Result<Vec<_>, _>>()?,
                native: m::NativeCellLocator {
                    request_record: problem.clone(),
                    raw_evidence: raw.clone(),
                    raw_cell_index: raw_index,
                },
                coverage: coverage.clone(),
            };
            if let Some(previous) = cells.insert(raw_index, cell.clone())
                && previous != cell
            {
                return Err(invalid("positive-projective sibling source cell mismatch"));
            }
            let definitions = bound.definitions(i);
            let functions = function_identity(&definitions, &renames)?;
            function_ids.push(functions.clone());
            let roots = bound.roots(i);
            let helpers = resolved::root_helpers(&roots);
            let mut sections = Vec::new();
            let mut section_ids = Vec::new();
            for (j, section) in chart.sections().iter().enumerate() {
                if !Arc::ptr_eq(section.owner(), owner)
                    || section.cell_index() != raw_index
                    || section.axis() >= chart.coordinate_order().len()
                    || !roots.owners().any(|(tag, p)| {
                        tag == section.tag() && Arc::ptr_eq(p, section.root_program())
                    })
                    || chart
                        .root_inputs()
                        .iter()
                        .filter(|(tag, input)| {
                            *tag == section.tag() && input.len() == section.coefficients().len()
                        })
                        .count()
                        != 1
                {
                    return Err(invalid(
                        "positive-projective normalized root owner association",
                    ));
                }
                let section_id = digest(
                    "threshold-positive-projective-normalized-section-v1",
                    &serde_json::json!({
                        "equation":canonicalize(section.equation()),
                        "prefix":canonicalize(section.prefix_image()),
                        "coefficients":section.coefficients().iter().map(&canonicalize).collect::<Vec<_>>(),
                        "bracket":[section.bracket().lower.to_string(),section.bracket().upper.to_string(),section.bracket().derivative_margin.to_string()], "orientation":section.orientation(),
                        "denominator":unit_identity(section.equation_denominator(),&canonicalize),
                        "minor":unit_identity(section.derivative_margin(),&canonicalize),
                        "endpoints":section.endpoints().iter().map(|u|unit_identity(u,&canonicalize)).collect::<Vec<_>>(),
                        "program":blake3::hash(section.root_program().bytes()).to_hex().to_string(),
                        "root_inputs":chart.root_inputs().iter().map(|(tag,input)|(tag.get_name(),input.iter().map(&canonicalize).collect::<Vec<_>>())).collect::<Vec<_>>(),
                    }),
                )?;
                section_ids.push(section_id.clone());
                for (k, branch) in section.branches().iter().enumerate() {
                    let symbol=Symbol::parse(format!("fastsecdec::positive_projective_root_role::chart_{i}_section_{j}_branch_{k}"),"").map_err(invalid)?;
                    if !symbol.is_exportable()
                        || symbols.contains(&symbol)
                        || owner.occupied_symbols().contains(&symbol)
                    {
                        return Err(invalid("positive-projective section role collision"));
                    }
                    let root_variable = m::NativeSymbolId(symbols.len());
                    symbols.push(symbol);
                    let equation = original_section_polynomial(
                        chart,
                        preparation.coordinates(),
                        section.axis(),
                        branch.original_polynomial(),
                        symbol,
                    )?;
                    sections.push(m::RootSection {
                        polynomial:push(&mut atoms,equation),root_variable,
                        preceding_coordinates:units.clone(),
                        domain:match branch.bound().index_domain {
                            symgcad::output::RootIndexDomain::Real=>m::RootDomain::RealAscending,
                            symgcad::output::RootIndexDomain::RealDescending=>m::RootDomain::RealDescending,
                            symgcad::output::RootIndexDomain::Positive=>m::RootDomain::Positive,
                        },
                        ordinal:branch.bound().index,
                        branch_certificate:digest("threshold-positive-projective-selected-branch-v1",&serde_json::json!({
                            "normalization":section_id,"side":format!("{:?}",branch.side()),"bound":branch.bound(),
                            "original":canonicalize(branch.original_polynomial()),"image":canonicalize(branch.image()),
                            "source_fibre":branch.source_fibre().map(ToString::to_string),"selected_anchor":canonicalize(branch.selected_anchor()),
                        }))?,
                    });
                }
            }
            let factor_proof = resolved::factor_margins(chart.factors(), &canonicalize)?;
            let incidence = digest(
                "threshold-positive-projective-source-incidence-v1",
                &serde_json::json!({
                    "branches":chart.branches().iter().map(|b|serde_json::json!({
                        "side":format!("{:?}",b.side()),"bound":b.bound(),
                        "original":canonicalize(b.original_polynomial()),"image":canonicalize(b.image()),
                        "source_fibre":b.source_fibre().map(ToString::to_string),"selected_anchor":canonicalize(b.selected_anchor()),
                    })).collect::<Vec<_>>(),
                    "coverage":resolved::factor_margins(chart.coverage_receipts(), &canonicalize)?,
                    "rewrites":chart.rewrite_receipts().iter().map(|r|serde_json::json!({
                        "origin":format!("{:?}",r.origin()),"original_pullback":canonicalize(r.original_pullback()),
                        "proposed":canonicalize(r.proposed()),"cleared_crossproduct":canonicalize(r.cleared_crossproduct()),
                    })).collect::<Vec<_>>(),
                }),
            )?;
            let geometry_proof = digest(
                "threshold-positive-projective-chart-geometry-v1",
                &(
                    &factor_proof,
                    &section_ids,
                    &incidence,
                    chart.source_axes(),
                    chart.endpoint_path(),
                    format!("{:?}", chart.sector_map()),
                ),
            )?;
            let mut terms = resolved::normalized_terms(
                chart.terms(),
                chart.factors(),
                preparation.terms(),
                &mut atoms,
                &geometry_proof,
            )?;
            let projected_numerators = numerator_transports(
                chart,
                preparation.terms(),
                preparation.coordinates(),
                &symbols,
                &canonicalize,
                &mut atoms,
            )?;
            // Provenance below retains full native proof receipts. Arithmetic
            // identity uses only canonical mathematical inputs and outputs;
            // discovery settings, proof method and serialized evidence are not
            // evaluator semantics.
            mathematical_charts.push(mathematical_chart(chart, &canonicalize));
            for numerator in projected_numerators {
                let term = terms
                    .iter_mut()
                    .find(|t| t.source_term == numerator.source_term)
                    .ok_or_else(|| invalid("numerator transport has no normalized source term"))?;
                term.numerators.push(numerator);
            }
            let endpoint = m::NormalizedEndpointV1 {
                regulator,
                terms,
                continuation_policy: actual_options.clone(),
                common_domain: common.clone(),
                numerator_functions: functions,
                root_helpers: helpers,
            };
            let endpoint_certificate = digest(
                "threshold-positive-projective-normal-form-v1",
                &(&endpoint, &geometry_proof, chart.endpoint_path()),
            )?;
            charts.push(m::EndpointChart {
                id: m::EndpointChartId(i),
                origin: m::EndpointOrigin::Cell(m::CellId(raw_index)),
                path: chart.endpoint_path().to_vec(),
                map: m::MapDescriptor::ResolvedEndpointV1 {
                    geometry: m::MapGeometry {
                        expressions: placeholder.clone(),
                        coordinates: units.clone(),
                        images: chart
                            .images()
                            .iter()
                            .map(|a| push(&mut atoms, a.clone()))
                            .collect(),
                        positive_measure: push(&mut atoms, chart.positive_measure().clone()),
                        orientation: if chart.orientation() > 0 {
                            m::Orientation::Forward
                        } else {
                            m::Orientation::Reversed
                        },
                        certificate: geometry_proof,
                    },
                    sections,
                    endpoint,
                },
                endpoint_certificate,
                causal_phase_certificate: digest(
                    "threshold-resolved-complete-phase-density-v1",
                    &canonicalize(&bound.charts()[i]),
                )?,
            });
            let kind = match record.kind {
                VectorKind::Stochastic {} => m::ContributionKind::Stochastic {
                    coordinates: units.clone(),
                },
                VectorKind::ZeroInLayout {} => m::ContributionKind::CertifiedZero {
                    certificate: digest(
                        "threshold-zero-in-requested-layout-v1",
                        &(&source, i, &orders),
                    )?,
                },
                VectorKind::Exact {} => {
                    return Err(invalid(
                        "positive-projective coefficient misclassified as exact",
                    ));
                }
            };
            contributions.push(m::Contribution {
                id: m::ContributionId(i),
                chart: m::EndpointChartId(i),
                group: m::GroupId(0),
                kind,
            });
        }
        let values = atoms.iter().map(&canonicalize).collect::<Vec<_>>();
        let expressions = digest("threshold-resolved-native-expression-values-v1", &values)?;
        if let m::MapDescriptor::RationalV1 { geometry } = &mut patch.map {
            geometry.expressions = expressions.clone();
        }
        for chart in &mut charts {
            if let m::MapDescriptor::ResolvedEndpointV1 { geometry, .. } = &mut chart.map {
                geometry.expressions = expressions.clone();
            }
        }
        let continued = digest(
            "threshold-positive-projective-continued-values-v1",
            &bound.charts().iter().map(&canonicalize).collect::<Vec<_>>(),
        )?;
        // Commit to whole-family content without copying its native expressions
        // into every restored parent's semantic string.
        let semantic = serde_json::to_string(&serde_json::json!({
            "projection_version":8,"recipe":"positive_projective_endpoint_v1","source":source,
            "policy":policy,"native_values":expressions,"continued":continued,"functions":function_ids,
            "common_domain":common,"anchor":anchor_math,
            "mathematical_charts":digest("threshold-positive-projective-mathematical-charts-v1", &mathematical_charts)?,
            "symbol_roles":symbols.iter().map(|s|canonicalize(&Atom::var(*s))).collect::<Vec<_>>(),
            "helpers":semantic_helpers,
        }))?;
        let manifest = m::LineageManifestV1 {
            version: 1,
            preparation: m::Preparation {
                runtime_chamber: None,
                source_identity: source.clone(),
                extent: m::SourceExtent::Full {
                    ordinary_geometry: None,
                },
                strategy: m::ResolvedStrategy::GcadFirst,
                actual_options,
                expressions,
                partition_certificate: coverage,
                original_coordinates: original,
                regulator,
                fiber: m::FixedRationalFiber {
                    physical_parameters: vec![],
                    bindings: vec![],
                    admission: digest(
                        "threshold-positive-projective-fixed-source-v1",
                        &(&source, &anchor),
                    )?,
                },
            },
            patches: vec![patch],
            cells: cells.into_values().collect(),
            endpoint_charts: charts,
            continuation_groups: vec![m::ContinuationGroup {
                id: m::GroupId(0),
                charts: (0..staged.len()).map(m::EndpointChartId).collect(),
                continuation: m::Continuation::EpsilonStripV1 {
                    certificate: common,
                },
            }],
            contributions,
        };
        Self::issued(manifest, atoms, symbols, semantic, orders)
    }
}

fn role(symbols: &[Symbol], symbol: Symbol) -> Result<m::NativeSymbolId, KernelError> {
    symbols
        .iter()
        .position(|s| *s == symbol)
        .map(m::NativeSymbolId)
        .ok_or_else(|| invalid("positive-projective source symbol role missing"))
}

fn original_section_polynomial(
    chart: &ResolvedPairChart,
    prepared: &[Symbol],
    axis: usize,
    polynomial: &Atom,
    root: Symbol,
) -> Result<Atom, KernelError> {
    let mut replacements = Vec::new();
    for (i, symbol) in chart.coordinate_order().iter().enumerate().take(axis + 1) {
        let image = if i == axis {
            Atom::var(root)
        } else {
            let index = prepared
                .iter()
                .position(|s| s == symbol)
                .ok_or_else(|| invalid("positive-projective named prefix role"))?;
            chart.images()[index].clone()
        };
        replacements.push(Replacement::new(
            Pattern::Literal(Atom::var(*symbol)),
            Pattern::Literal(image),
        ));
    }
    Ok(polynomial.replace_multiple(&replacements))
}

fn unit_identity(
    unit: &crate::threshold::regularization::projective_family::ClosedUnit,
    canonicalize: &impl Fn(&Atom) -> String,
) -> serde_json::Value {
    serde_json::json!({
        "expression":canonicalize(unit.expression()),"sign":unit.sign(),"margin":unit.margin().to_string(),
        "polynomial":unit.polynomial().to_expression().to_canonical_string(),
        "constraints":unit.constraints().iter().map(|p|p.to_expression().to_canonical_string()).collect::<Vec<_>>(),
        "certificate":unit.certificate(),
    })
}

fn numerator_transports(
    chart: &ResolvedPairChart,
    source_terms: &[crate::parametric::ParametricTerm],
    prepared: &[Symbol],
    symbols: &[Symbol],
    canonicalize: &impl Fn(&Atom) -> String,
    atoms: &mut Vec<Atom>,
) -> Result<Vec<m::NormalizedNumeratorV1>, KernelError> {
    let source_coordinates = prepared
        .iter()
        .map(|s| role(symbols, *s))
        .collect::<Result<Vec<_>, _>>()?;
    let mut seen = BTreeSet::new();
    chart.numerator_transport().iter().map(|n| {
        let term = source_terms.get(n.source_index())
            .ok_or_else(|| invalid("numerator transport source term"))?;
        if n.degrees().len() != prepared.len()
            || n.powers().len() != chart.images().len()
            || n.template_roles().len() != n.finite_images().len()
            || n.factor_indices().windows(2).any(|w| w[0] >= w[1])
            || n.factor_indices().iter().any(|index| {
                !seen.insert((n.source_index(), *index))
                    || term.factors().get(*index).is_none_or(|f| f.role() != crate::parametric::FactorRole::Polynomial)
            })
        {
            return Err(invalid("numerator transport dimension/source-factor association"));
        }
        let source = digest("threshold-positive-projective-numerator-source-v1", &canonicalize(n.source()))?;
        let template = digest("threshold-positive-projective-numerator-template-v1", &canonicalize(n.homogeneous_template()))?;
        let denominator_proofs = n.denominators().iter()
            .map(|f| resolved::factor_margins(std::slice::from_ref(f), canonicalize))
            .collect::<Result<Vec<_>, _>>()?;
        let transport = digest("threshold-positive-projective-numerator-transport-v1", &serde_json::json!({
            "term":n.source_index(),"factors":n.factor_indices(),"source_factor_count":term.factors().len(),
            "source":source,"template":template,"regular":canonicalize(n.regular()),
            "source_coordinates":prepared.iter().map(|s|s.get_name()).collect::<Vec<_>>(),
            "template_roles":n.template_roles().iter().map(|s|canonicalize(&Atom::var(*s))).collect::<Vec<_>>(),
            "finite_images":n.finite_images().iter().map(canonicalize).collect::<Vec<_>>(),
            "degrees":n.degrees(),"powers":n.powers(),"denominator_proofs":denominator_proofs,
        }))?;
        Ok(m::NormalizedNumeratorV1 {
            source_term:n.source_index(),source_factor_count:term.factors().len(),source_factors:n.factor_indices().to_vec(),
            source_coordinates:source_coordinates.clone(),source,template,regular:push(atoms,n.regular().clone()),
            degrees:n.degrees().to_vec(),powers:n.powers().to_vec(),transport,denominator_proofs,
        })
    }).collect()
}

#[cfg(test)]
mod tests;

fn semantic_role(name: &str) -> Result<Symbol, KernelError> {
    Symbol::parse(format!("fastsecdec::threshold_semantic::{name}"), "").map_err(invalid)
}

fn function_identity(
    definitions: &[crate::threshold::continued::Definition],
    renames: &BTreeMap<Symbol, Symbol>,
) -> Result<m::Digest, KernelError> {
    let mut rows = Vec::with_capacity(definitions.len());
    for definition in definitions {
        let mut local = renames.clone();
        for (i, formal) in definition.formals.iter().enumerate() {
            local.insert(*formal, semantic_role(&format!("function_argument_{i}"))?);
        }
        rows.push((
            canonical(&Atom::var(definition.head), renames),
            definition
                .tags
                .iter()
                .map(|a| canonical(a, renames))
                .collect::<Vec<_>>(),
            definition.formals.len(),
            canonical(&definition.body, &local),
            &definition.orders,
            definition.always_inline,
        ));
    }
    digest("threshold-positive-projective-functions-v1", &rows)
}

fn mathematical_chart(
    chart: &ResolvedPairChart,
    canonicalize: &impl Fn(&Atom) -> String,
) -> serde_json::Value {
    let branch = |b: &crate::threshold::regularization::projective_pair_family::BranchReceipt| {
        serde_json::json!({
            "side":format!("{:?}",b.side()),
            "polynomial":canonicalize(b.original_polynomial()),
            "index_domain":format!("{:?}",b.bound().index_domain),"ordinal":b.bound().index,
            "image":canonicalize(b.image()),
        })
    };
    serde_json::json!({
        "cell":chart.cell_index(),"path":chart.endpoint_path(),
        "coordinate_order":chart.coordinate_order().iter().map(|s|canonicalize(&Atom::var(*s))).collect::<Vec<_>>(),
        "images":chart.images().iter().map(canonicalize).collect::<Vec<_>>(),
        "measure":canonicalize(chart.positive_measure()),"orientation":chart.orientation(),
        "branches":chart.branches().iter().map(&branch).collect::<Vec<_>>(),
        "sections":chart.sections().iter().map(|s|serde_json::json!({
            "equation":canonicalize(s.equation()),"prefix":canonicalize(s.prefix_image()),
            "coefficients":s.coefficients().iter().map(canonicalize).collect::<Vec<_>>(),
            "branches":s.branches().iter().map(&branch).collect::<Vec<_>>(),
            "program":s.root_program().semantic_identity(),
        })).collect::<Vec<_>>(),
        "root_inputs":chart.root_inputs().iter().map(|(tag,input)|(canonicalize(&Atom::var(*tag)),input.iter().map(canonicalize).collect::<Vec<_>>())).collect::<Vec<_>>(),
        "numerators":chart.numerator_transport().iter().map(|n|serde_json::json!({
            "term":n.source_index(),"factors":n.factor_indices(),"source":canonicalize(n.source()),
            "template":canonicalize(n.homogeneous_template()),"regular":canonicalize(n.regular()),
            "template_roles":n.template_roles().iter().map(|s|canonicalize(&Atom::var(*s))).collect::<Vec<_>>(),
            "finite_images":n.finite_images().iter().map(canonicalize).collect::<Vec<_>>(),
            "degrees":n.degrees(),"powers":n.powers(),
        })).collect::<Vec<_>>(),
    })
}

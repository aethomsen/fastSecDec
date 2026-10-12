//! Native projection of a complete resolved projective atlas. Arithmetic
//! descriptors retain the actual full maps; they never issue geometric proofs.
use super::algebraic::{canonical, digest, push};
use super::*;
use crate::threshold::{
    records::{StagedVector, VectorKind},
    regularization::projective_family::{ContinuedProjectiveFamily, FactorOrigin},
};
use std::collections::BTreeSet;
use symbolica::atom::AtomCore;

impl ThresholdMetadata {
    pub(super) fn from_projective_family(
        bound: &ContinuedProjectiveFamily<'_>,
        staged: &[StagedVector],
        orders: Vec<i32>,
    ) -> Result<Self, KernelError> {
        let certificate = bound.certificate();
        let owner = certificate.owner();
        let request = owner.request();
        let source_patch = source_patch::SourcePatch::from_request(request)?;
        let coordinates = certificate.coordinates();
        if !request.kinematics().runtime_parameters.is_empty()
            || coordinates.len() != source_patch.coordinates.len()
            || staged.len() != certificate.charts().len()
            || staged.iter().any(|r| !r.algebraic)
        {
            return Err(invalid(
                "resolved projective family/source record association",
            ));
        }
        let inventory = certificate
            .charts()
            .iter()
            .map(|c| c.cell_index())
            .collect::<BTreeSet<_>>();
        if inventory != owner.cells().map(|c| c.index()).collect::<BTreeSet<_>>() {
            return Err(invalid("resolved projective atlas omits a verified cell"));
        }
        let mut symbols = request.input().parameters().to_vec();
        let original = (0..symbols.len())
            .map(m::NativeSymbolId)
            .collect::<Vec<_>>();
        let prepared = source_patch
            .coordinates
            .iter()
            .map(|s| {
                symbols
                    .iter()
                    .position(|p| p == s)
                    .map(m::NativeSymbolId)
                    .ok_or_else(|| invalid("prepared role missing from original input"))
            })
            .collect::<Result<Vec<_>, KernelError>>()?;
        let regulator = m::NativeSymbolId(symbols.len());
        symbols.push(request.input().regulator());
        let physical = request
            .kinematics()
            .exact_values
            .keys()
            .copied()
            .collect::<Vec<_>>();
        let physical_ids = physical
            .iter()
            .map(|s| {
                let id = m::NativeSymbolId(symbols.len());
                symbols.push(*s);
                id
            })
            .collect::<Vec<_>>();
        let units = coordinates
            .iter()
            .map(|s| {
                let id = m::NativeSymbolId(symbols.len());
                symbols.push(*s);
                id
            })
            .collect::<Vec<_>>();
        let renames = coordinates
            .iter()
            .enumerate()
            .map(|(i, s)| {
                Symbol::parse(format!("fastsecdec::threshold_semantic::unit_{i}"), "")
                    .map(|target| (*s, target))
                    .map_err(invalid)
            })
            .collect::<Result<BTreeMap<_, _>, KernelError>>()?;
        let canonicalize = |a: &Atom| canonical(a, &renames);
        let options = bound.options();
        let policy = serde_json::json!({"mode":options.mode,"subtraction":options.subtraction,"max_order":options.max_order,"max_subtractions_per_axis":options.max_subtractions_per_axis,"max_subtraction_terms":options.max_subtraction_terms,"coefficient_expansion":options.coefficient_expansion});
        let actual_options = digest("threshold-resolved-continuation-policy-v1", &policy)?;
        let strip = certificate.convergence_strip();
        let witness = certificate.prefactor_witness();
        let common = digest(
            "threshold-resolved-common-domain-v1",
            &serde_json::json!({
                "strip":[strip.lower().map(ToString::to_string),strip.upper().map(ToString::to_string)],
                "witness":witness.rational_witness().epsilon().to_string(),
                "gamma":witness.gamma_witnesses().iter().map(|g|(g.term(),g.factor(),g.argument_value().to_string(),g.pole_distance().map(ToString::to_string))).collect::<Vec<_>>()
            }),
        )?;
        let definitions = bound.definitions();
        let functions = digest(
            "threshold-resolved-functions-v1",
            &definitions
                .iter()
                .map(|d| {
                    (
                        d.head.get_name(),
                        d.tags.iter().map(&canonicalize).collect::<Vec<_>>(),
                        d.formals.iter().map(|s| s.get_name()).collect::<Vec<_>>(),
                        canonicalize(&d.body),
                        &d.orders,
                        d.always_inline,
                    )
                })
                .collect::<Vec<_>>(),
        )?;
        let roots = certificate.callback_scope();
        let mut helpers = roots
            .saved()
            .into_iter()
            .map(|(_, bytes)| m::Digest(blake3::hash(&bytes).to_hex().to_string()))
            .collect::<Vec<_>>();
        helpers.sort();
        helpers.dedup();
        let source = m::Digest(request.source_identity().map_err(invalid)?);
        let raw = digest("threshold-native-solve-evidence-v1", owner.native_result())?;
        let problem = digest("threshold-native-problem-v1", request.problem())?;
        let coverage = digest(
            "threshold-resolved-complete-cell-inventory-v1",
            &(&raw, &inventory),
        )?;
        let mut atoms = Vec::new();
        let placeholder = m::Digest("0".repeat(64));
        let bindings = physical_ids
            .iter()
            .zip(&physical)
            .map(|(s, v)| m::RationalBinding {
                symbol: *s,
                value: push(
                    &mut atoms,
                    Atom::num(request.kinematics().exact_values[v].clone()),
                ),
            })
            .collect::<Vec<_>>();
        let mut patch = m::Patch {
            id: m::PatchId(0),
            source: m::SourceOrigin::OriginalInput,
            coverage: coverage.clone(),
            map: m::MapDescriptor::RationalV1 {
                geometry: m::MapGeometry {
                    expressions: placeholder.clone(),
                    coordinates: prepared.clone(),
                    images: source_patch
                        .images
                        .iter()
                        .map(|a| push(&mut atoms, a.clone()))
                        .collect(),
                    positive_measure: push(&mut atoms, source_patch.positive_measure.clone()),
                    orientation: m::Orientation::Forward,
                    certificate: coverage.clone(),
                },
            },
        };
        let mut cells = BTreeMap::new();
        let mut charts = Vec::new();
        let mut contributions = Vec::new();
        let mut paths = BTreeSet::new();
        for (i, (chart, record)) in certificate.charts().iter().zip(staged).enumerate() {
            if record.chart != i
                || chart.images().len() != prepared.len()
                || !paths.insert((chart.cell_index(), chart.endpoint_path()))
            {
                return Err(invalid("resolved chart order/dimension/path association"));
            }
            let raw_index = chart.cell_index();
            let cell_order = chart
                .geometry()
                .source()
                .axes()
                .iter()
                .map(|axis| {
                    symbols
                        .iter()
                        .position(|s| *s == axis.symbol())
                        .map(m::NativeSymbolId)
                        .ok_or_else(|| invalid("native cell coordinate missing from source roles"))
                })
                .collect::<Result<Vec<_>, KernelError>>()?;
            let cell = m::Cell {
                id: m::CellId(raw_index),
                patch: m::PatchId(0),
                coordinate_order: cell_order,
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
                return Err(invalid("resolved sibling source cell mismatch"));
            }
            let mut sections = Vec::new();
            for (j, section) in chart.geometry().sections().iter().enumerate() {
                if section.axis() != 0 {
                    return Err(invalid(
                        "resolved constant section unexpectedly has prefix arguments",
                    ));
                }
                let root_symbol = Symbol::parse(
                    format!("fastsecdec::resolved_root_role::chart_{i}_root_{j}"),
                    "",
                )
                .map_err(invalid)?;
                if !root_symbol.is_exportable() || symbols.contains(&root_symbol) {
                    return Err(invalid("resolved root formal collides with native roles"));
                }
                let root_variable = m::NativeSymbolId(symbols.len());
                symbols.push(root_symbol);
                let original_root = chart.geometry().source().axes()[section.axis()].symbol();
                let equation = section
                    .equation()
                    .replace(Atom::var(original_root))
                    .with(Atom::var(root_symbol));
                sections.push(m::RootSection {
                    polynomial: push(&mut atoms, equation),
                    root_variable,
                    preceding_coordinates: vec![],
                    domain: match section.bound().index_domain {
                        symgcad::output::RootIndexDomain::Real => m::RootDomain::RealAscending,
                        symgcad::output::RootIndexDomain::RealDescending => {
                            m::RootDomain::RealDescending
                        }
                        symgcad::output::RootIndexDomain::Positive => m::RootDomain::Positive,
                    },
                    ordinal: section.bound().index,
                    branch_certificate: digest(
                        "threshold-resolved-constant-section-v1",
                        &(
                            section.bound(),
                            section.derivative_certificate(),
                            section.endpoint_certificates(),
                        ),
                    )?,
                });
            }
            let geometry_proof = digest("threshold-resolved-closed-factor-margins-v1",&chart.factors().iter().map(|f| {
                let unit = |u: &crate::threshold::regularization::projective_family::ClosedUnit| {
                    serde_json::json!({"expression":canonicalize(u.expression()),"sign":u.sign(),"margin":u.margin().to_string(),"polynomial":u.polynomial().to_expression().to_canonical_string(),"constraints":u.constraints().iter().map(|p|p.to_expression().to_canonical_string()).collect::<Vec<_>>(),"certificate":u.certificate()})
                };
                serde_json::json!({"role":format!("{:?}",f.origin()),"source":canonicalize(f.source()),"powers":f.powers(),"numerator":unit(f.numerator()),"denominator":unit(f.denominator())})
            }).collect::<Vec<_>>())?;
            let terms = chart
                .terms()
                .iter()
                .map(|t| {
                    let mut factors = Vec::new();
                    for f in chart.factors() {
                        if let FactorOrigin::Factor { term, factor } = f.origin()
                            && term == t.source_index()
                        {
                            let exponent = request
                                .projective_preparation()
                                .and_then(|p| p.terms().get(term))
                                .and_then(|t| t.factors().get(factor))
                                .map(|f| f.exponent())
                                .ok_or_else(|| invalid("resolved prepared factor association"))?;
                            factors.push(m::NormalizedFactorV1 {
                                source_term: term,
                                source_factor: factor,
                                source: push(&mut atoms, f.source().clone()),
                                exponent: push(&mut atoms, exponent.clone()),
                                certificate: geometry_proof.clone(),
                            });
                        }
                    }
                    Ok(m::NormalizedTermV1 {
                        source_term: t.source_index(),
                        prefactor: push(&mut atoms, t.prefactor().clone()),
                        regular: push(&mut atoms, t.regular().clone()),
                        powers: t
                            .powers()
                            .iter()
                            .map(|p| push(&mut atoms, p.clone()))
                            .collect(),
                        factors,
                    })
                })
                .collect::<Result<Vec<_>, KernelError>>()?;
            let endpoint = m::NormalizedEndpointV1 {
                regulator,
                terms,
                continuation_policy: actual_options.clone(),
                common_domain: common.clone(),
                numerator_functions: functions.clone(),
                root_helpers: helpers.clone(),
            };
            let endpoint_certificate = digest(
                "threshold-resolved-normal-form-v1",
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
                        "resolved algebraic coefficient misclassified as exact",
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
        let semantic = serde_json::to_string(
            &serde_json::json!({"projection_version":7,"recipe":"resolved_projective_endpoint_v1","source":source,"policy":policy,"native_values":values,"continued":bound.charts().iter().map(canonicalize).collect::<Vec<_>>(),"functions":functions,"common_domain":common}),
        )?;
        let manifest = m::LineageManifestV1 {
            version: 1,
            preparation: m::Preparation {
                runtime_chamber: None,
                source_identity: source,
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
                    physical_parameters: physical_ids,
                    bindings,
                    admission: digest(
                        "threshold-resolved-fixed-fiber-v1",
                        &request
                            .kinematics()
                            .exact_values
                            .iter()
                            .map(|(s, v)| (s.get_name(), v.to_string()))
                            .collect::<Vec<_>>(),
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

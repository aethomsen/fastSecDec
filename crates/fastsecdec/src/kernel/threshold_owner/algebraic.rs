//! Projection of the checked fixed or uniform secant family into local lineage.
//! No saved descriptor can construct the geometric owner consumed here.
use super::*;
use crate::threshold::{
    continued::ContinuedSource,
    gcad::DomainOrigin,
    records::{StagedVector, VectorKind},
    regularization::secant::{ChartGeometry, ContinuedFamily, FactorKind},
};
use symbolica::{
    atom::{AtomCore, AtomView, FunctionBuilder},
    id::{Pattern, Replacement},
};
fn digest(domain: &str, value: &impl serde::Serialize) -> Result<m::Digest, KernelError> {
    let mut h = blake3::Hasher::new();
    h.update(domain.as_bytes());
    serde_json::to_writer(&mut h, value)?;
    Ok(m::Digest(h.finalize().to_hex().to_string()))
}
fn canonical(a: &Atom, renames: &BTreeMap<Symbol, Symbol>) -> String {
    a.replace_map_bottom_up(|node, _, out| match node {
        AtomView::Var(v) => {
            if let Some(s) = renames.get(&v.get_symbol()) {
                **out = Atom::var(*s)
            }
        }
        AtomView::Fun(f) => {
            if let Some(s) = renames.get(&f.get_symbol()) {
                let mut b = FunctionBuilder::new(*s);
                for a in f.iter() {
                    b = b.add_arg(a)
                }
                **out = b.finish()
            }
        }
        _ => {}
    })
    .to_canonical_string()
}
fn push(atoms: &mut Vec<Atom>, a: Atom) -> m::NativeAtomId {
    let i = atoms.len();
    atoms.push(a);
    m::NativeAtomId(i)
}
impl ThresholdMetadata {
    pub(crate) fn from_source(
        source: ContinuedSource<'_>,
        staged: &[StagedVector],
        orders: Vec<i32>,
    ) -> Result<Self, KernelError> {
        match source {
            ContinuedSource::Rational(b) => Self::from_bound(b, staged, orders),
            ContinuedSource::Algebraic(b) => Self::from_family(b, staged, orders),
        }
    }
    fn from_family(
        bound: &ContinuedFamily<'_>,
        staged: &[StagedVector],
        orders: Vec<i32>,
    ) -> Result<Self, KernelError> {
        let certificate = bound.certificate();
        let request = certificate.owner().request();
        let admitted_origin = match request.domain().origin() {
            DomainOrigin::NativeUnitCube => true,
            DomainOrigin::AffineProjective { .. } => {
                request.input().parameters().len() == 2
                    && request.domain().coordinates().len() == 1
                    && certificate.parameter_chamber().is_some()
                    && certificate.roots().len() == 2
            }
            _ => false,
        };
        if !admitted_origin
            || staged.len() != certificate.charts().len()
            || staged.iter().any(|r| !r.algebraic)
        {
            return Err(invalid(
                "incomplete or unsupported certified fixed secant family",
            ));
        }
        let chamber = certificate.parameter_chamber();
        let runtime = certificate.runtime_parameters();
        if runtime != request.kinematics().runtime_parameters
            || chamber.is_some() != !runtime.is_empty()
        {
            return Err(invalid("runtime family lacks its uniform chamber owner"));
        }
        let original_symbols = request.input().parameters();
        let source_patch = super::source_patch::SourcePatch::from_request(request)?;
        let prepared_symbols = source_patch.coordinates;
        let coordinates = certificate.coordinates();
        let integration_axes =
            if let Some(chamber) = chamber {
                if !Arc::ptr_eq(chamber.decomposition(), certificate.owner())
                    || chamber.runtime_parameters() != runtime
                    || chamber.layout().units() != coordinates
                    || chamber
                        .cell_indices()
                        .collect::<std::collections::BTreeSet<_>>()
                        != certificate
                            .charts()
                            .iter()
                            .map(|c| c.geometry().source().cell_index())
                            .collect()
                    || chamber.integration_axes().len() != prepared_symbols.len()
                    || chamber.integration_axes().iter().zip(prepared_symbols).any(
                        |(axis, symbol)| chamber.layout().raw_symbols().get(*axis) != Some(symbol),
                    )
                {
                    return Err(invalid(
                        "uniform chamber complete family/source role association",
                    ));
                }
                chamber.integration_axes().to_vec()
            } else {
                (0..prepared_symbols.len()).collect::<Vec<_>>()
            };
        let mut symbols = original_symbols.to_vec();
        let original = (0..symbols.len())
            .map(m::NativeSymbolId)
            .collect::<Vec<_>>();
        let prepared = prepared_symbols
            .iter()
            .map(|s| {
                original_symbols
                    .iter()
                    .position(|p| p == s)
                    .map(m::NativeSymbolId)
                    .ok_or_else(|| invalid("prepared coordinate is not an original native role"))
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
        let runtime_ids = runtime
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
        let paired = certificate.roots().len() == 2;
        let branch_symbols = if paired {
            let pair = [
                symbolica::symbol!("fastsecdec::threshold_pair::lower_formal"),
                symbolica::symbol!("fastsecdec::threshold_pair::upper_formal"),
            ];
            if pair.iter().any(|s| symbols.contains(s)) {
                return Err(invalid(
                    "paired formal root collides with source/unit symbol",
                ));
            }
            pair.into_iter()
                .map(|s| {
                    let id = m::NativeSymbolId(symbols.len());
                    symbols.push(s);
                    (s, id)
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let mut renames = BTreeMap::new();
        for (i, s) in coordinates.iter().enumerate() {
            renames.insert(
                *s,
                Symbol::parse(format!("fastsecdec::threshold_semantic::unit_{i}"), "")
                    .map_err(invalid)?,
            );
        }
        for (role, (symbol, _)) in branch_symbols.iter().enumerate() {
            renames.insert(
                *symbol,
                Symbol::parse(format!("fastsecdec::threshold_semantic::branch_{role}"), "")
                    .map_err(invalid)?,
            );
        }
        let definitions = bound.definitions();
        let head = definitions
            .iter()
            .find(|d| d.orders.is_none())
            .ok_or_else(|| invalid("missing numerator definition"))?
            .head;
        if definitions
            .iter()
            .any(|d| d.orders.is_none() && d.head != head)
        {
            return Err(invalid("multiple numerator roles"));
        }
        renames.insert(
            head,
            symbolica::symbol!("fastsecdec::threshold_semantic::numerator"),
        );
        let mut semantic_helpers = std::collections::BTreeSet::new();
        for (tag, helper) in certificate.callback_scope().owners() {
            let identity = helper.semantic_identity();
            semantic_helpers.insert(m::Digest(identity.to_owned()));
            renames.insert(
                helper.transport_symbol(),
                Symbol::parse(
                    format!("fastsecdec::threshold_semantic::root_contract_{identity}"),
                    "",
                )
                .map_err(invalid)?,
            );
            renames.insert(
                tag,
                Symbol::parse(
                    format!("fastsecdec::threshold_semantic::root_owner_{identity}"),
                    "",
                )
                .map_err(invalid)?,
            );
        }
        let mut function_roles = BTreeMap::new();
        for d in definitions {
            let (slot, mixed) = if let Some(mixed) = &d.orders {
                if d.head != Symbol::DERIVATIVE
                    || d.tags.len() != mixed.len() + 2
                    || d.tags[mixed.len()] != Atom::var(head)
                    || d.tags.iter().zip(mixed).any(|(a, n)| a != &Atom::num(*n))
                {
                    return Err(invalid("mixed derivative owner association"));
                }
                (
                    usize::try_from(d.tags.last().unwrap().as_view()).map_err(invalid)?,
                    mixed.clone(),
                )
            } else {
                if d.head != head || d.tags.len() != 1 {
                    return Err(invalid("numerator role"));
                }
                (
                    usize::try_from(d.tags[0].as_view()).map_err(invalid)?,
                    vec![0; d.formals.len() + 1],
                )
            };
            if function_roles.insert((slot, mixed), d).is_some() {
                return Err(invalid("duplicate numerator definition role"));
            }
        }
        let functions = digest(
            "threshold-secant-functions-v1",
            &function_roles
                .iter()
                .map(|(role, d)| (role, canonical(&d.body, &renames)))
                .collect::<Vec<_>>(),
        )?;
        let helpers = certificate
            .callback_scope()
            .saved()
            .iter()
            .map(|(_, bytes)| m::Digest(blake3::hash(bytes).to_hex().to_string()))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let options = bound.options();
        let policy = serde_json::json!({"mode":options.mode,"subtraction":options.subtraction,"max_order":options.max_order,"max_subtractions_per_axis":options.max_subtractions_per_axis,"max_subtraction_terms":options.max_subtraction_terms,"coefficient_expansion":options.coefficient_expansion});
        let actual_options = digest("threshold-secant-continuation-policy-v1", &policy)?;
        let common = digest(
            "threshold-secant-common-domain-v1",
            &serde_json::json!({"strip":[certificate.strip().lower().map(ToString::to_string),certificate.strip().upper().map(ToString::to_string)],"witness":certificate.witness().rational_witness().epsilon().to_string(),"gamma":certificate.witness().gamma_witnesses().iter().map(|g|(g.term(),g.factor(),g.argument_value().to_string(),g.pole_distance().map(ToString::to_string))).collect::<Vec<_>>() }),
        )?;
        let source = m::Digest(request.source_identity().map_err(invalid)?);
        let raw = digest(
            "threshold-native-solve-evidence-v1",
            certificate.owner().native_result(),
        )?;
        let problem = digest("threshold-native-problem-v1", request.problem())?;
        let coverage = digest(
            if paired {
                "threshold-paired-secant-complete-cell-inventory-v1"
            } else {
                "threshold-secant-complete-cell-inventory-v1"
            },
            &(
                raw.clone(),
                certificate
                    .charts()
                    .iter()
                    .map(|c| c.geometry().source().cell_index())
                    .collect::<Vec<_>>(),
            ),
        )?;
        let mut atoms = Vec::new();
        let one = push(&mut atoms, source_patch.positive_measure.clone());
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
        let patch_images = source_patch
            .images
            .iter()
            .map(|a| push(&mut atoms, a.clone()))
            .collect::<Vec<_>>();
        // Temporary digest is replaced after the complete native table is built.
        let placeholder = m::Digest("0".repeat(64));
        let patch = m::Patch {
            id: m::PatchId(0),
            source: m::SourceOrigin::OriginalInput,
            map: m::MapDescriptor::RationalV1 {
                geometry: m::MapGeometry {
                    expressions: placeholder.clone(),
                    coordinates: prepared.clone(),
                    images: patch_images,
                    positive_measure: one,
                    orientation: m::Orientation::Forward,
                    certificate: coverage.clone(),
                },
            },
            coverage: coverage.clone(),
        };
        let mut cells = Vec::new();
        let mut charts = Vec::new();
        let mut contributions = Vec::new();
        for (i, (chart, record)) in certificate.charts().iter().zip(staged).enumerate() {
            if record.chart != i {
                return Err(invalid("secant staged chart order"));
            }
            let g = chart.geometry();

            let raw_index = g.source().cell_index();
            let native_cell = m::Cell {
                id: m::CellId(raw_index),
                patch: m::PatchId(0),
                native: m::NativeCellLocator {
                    request_record: problem.clone(),
                    raw_evidence: raw.clone(),
                    raw_cell_index: raw_index,
                },
                coordinate_order: prepared.clone(),
                coverage: coverage.clone(),
            };
            if let Some(existing) = cells.iter().find(|c: &&m::Cell| c.id == native_cell.id) {
                if existing != &native_cell {
                    return Err(invalid("repeated native cell has different association"));
                }
            } else {
                cells.push(native_cell);
            }
            let mut images = coordinates
                .iter()
                .take(coordinates.len() - 1)
                .map(|s| push(&mut atoms, Atom::var(*s)))
                .collect::<Vec<_>>();
            images.push(push(&mut atoms, chart.image().clone()));
            let mut terms = Vec::new();
            for t in chart.terms() {
                let factors=t.ledger().iter().map(|f|{
                    let cert=match f.kind(){FactorKind::Section{scale}=>match g {
                        ChartGeometry::Single(g)=>digest("threshold-secant-factor-v1",&serde_json::json!({"scale":scale.to_string(),"secant":g.secant().to_expression().to_canonical_string(),"endpoint_secant":g.endpoint_secant().to_expression().to_canonical_string(),"width_powers":g.width().powers(),"width_residual":g.width().residual().to_expression().to_canonical_string(),"width_certificate":g.width().certificate(),"segment_derivative":g.whole_segment().derivative_certificate()}))?,
                        ChartGeometry::Paired(g)=>digest("threshold-paired-secant-factor-v1",&serde_json::json!({"scale":scale.to_string(),"second_secant":g.pair().second_secant().to_expression().to_canonical_string(),"unit":g.pair().unit_certificate(),"gap":g.pair().gap_certificate(),"widths":g.pair().width_certificates(),"part":format!("{:?}",g.part()),"sections":g.pair().sections().iter().map(|s|(s.bound(),s.derivative_certificate(),s.endpoint_certificates())).collect::<Vec<_>>()}))?,
                    },FactorKind::ClosedUnit{certificate}=>digest("threshold-secant-additional-unit-v1",certificate.as_ref())?,FactorKind::PolynomialNumerator=>digest("threshold-exact-polynomial-numerator-v1",&canonical(f.source(),&renames))?};
                    Ok(m::NormalizedFactorV1{source_term:f.indices().0,source_factor:f.indices().1,source:push(&mut atoms,f.source().clone()),exponent:push(&mut atoms,f.exponent().clone()),certificate:cert})
                }).collect::<Result<Vec<_>,KernelError>>()?;
                terms.push(m::NormalizedTermV1 {
                    source_term: t.source_term(),
                    prefactor: push(&mut atoms, t.prefactor().clone()),
                    regular: push(&mut atoms, t.regular().clone()),
                    powers: t
                        .powers()
                        .iter()
                        .map(|a| push(&mut atoms, a.clone()))
                        .collect(),
                    factors,
                });
            }
            let roots = g
                .sections()
                .iter()
                .enumerate()
                .map(|(role, section)| {
                    let mut equation = section.equation().replace_multiple(
                        prepared_symbols
                            .iter()
                            .zip(coordinates)
                            .zip(&integration_axes)
                            .filter(|(_, axis)| **axis < section.axis())
                            .map(|((s, u), _)| {
                                Replacement::new(
                                    Pattern::Literal(Atom::var(*s)),
                                    Pattern::Literal(Atom::var(*u)),
                                )
                            }),
                    );
                    let integration_axis = integration_axes
                        .iter()
                        .position(|axis| *axis == section.axis())
                        .ok_or_else(|| invalid("selected root is not an integration axis"))?;
                    let root_variable = if paired {
                        equation = equation
                            .replace(Atom::var(prepared_symbols[integration_axis]))
                            .with(Atom::var(branch_symbols[role].0));
                        branch_symbols[role].1
                    } else {
                        prepared[integration_axis]
                    };
                    let root = m::RootSection {
                        polynomial: push(&mut atoms, equation),
                        root_variable,
                        preceding_coordinates: units
                            .iter()
                            .zip(&integration_axes)
                            .filter_map(|(u, axis)| (*axis < section.axis()).then_some(*u))
                            .collect(),
                        domain: match section.bound().index_domain {
                            symgcad::output::RootIndexDomain::Real => m::RootDomain::RealAscending,
                            symgcad::output::RootIndexDomain::Positive => m::RootDomain::Positive,
                            symgcad::output::RootIndexDomain::RealDescending => {
                                m::RootDomain::RealDescending
                            }
                        },
                        ordinal: section.bound().index,
                        branch_certificate: digest(
                            "threshold-regular-section-association-v1",
                            &(
                                section.bound(),
                                section.derivative_certificate(),
                                section.endpoint_certificates(),
                                &helpers,
                            ),
                        )?,
                    };
                    Ok(root)
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
            let endpoint_certificate = if paired {
                digest(
                    "threshold-normalized-paired-secant-endpoint-v1",
                    &(
                        &endpoint,
                        &staged[i].coefficient_zeros,
                        bound
                            .face_reductions()
                            .iter()
                            .filter(|r| r.chart == i)
                            .map(|r| {
                                (
                                    r.definition,
                                    r.role,
                                    canonical(&r.body, &renames),
                                    canonical(&r.equation, &renames),
                                    canonical(&r.quotient, &renames),
                                    canonical(&r.remainder, &renames),
                                )
                            })
                            .collect::<Vec<_>>(),
                    ),
                )?
            } else {
                digest("threshold-normalized-secant-endpoint-v1", &endpoint)?
            };
            charts.push(m::EndpointChart {
                id: m::EndpointChartId(i),
                origin: m::EndpointOrigin::Cell(m::CellId(raw_index)),
                path: vec![i],
                map: m::MapDescriptor::RegularSecantV1 {
                    geometry: m::MapGeometry {
                        expressions: placeholder.clone(),
                        coordinates: units.clone(),
                        images,
                        positive_measure: push(&mut atoms, chart.positive_measure().clone()),
                        orientation: if g.reversed() {
                            m::Orientation::Reversed
                        } else {
                            m::Orientation::Forward
                        },
                        certificate: coverage.clone(),
                    },
                    sections: roots,
                    endpoint,
                },
                endpoint_certificate,
                causal_phase_certificate: digest(
                    "threshold-secant-complete-phase-density-v1",
                    &canonical(&bound.charts()[i], &renames),
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
                        "algebraic coefficient misclassified as exact scalar",
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
        cells.sort_by_key(|c| c.id);
        let values = atoms
            .iter()
            .map(|a| canonical(a, &renames))
            .collect::<Vec<_>>();
        let expressions = digest("threshold-secant-native-expression-values-v1", &values)?;
        let mut patch = patch;
        if let m::MapDescriptor::RationalV1 { geometry } = &mut patch.map {
            geometry.expressions = expressions.clone();
        }
        for chart in &mut charts {
            if let m::MapDescriptor::RegularSecantV1 { geometry, .. } = &mut chart.map {
                geometry.expressions = expressions.clone();
            }
        }
        let mut semantic_projection = serde_json::json!({"projection_version":if paired {4}else{3},"recipe":if paired {"regular_paired_secant_v1"}else{"regular_secant_v1"},"source":source,"policy":policy,"native_values":digest("threshold-secant-canonical-values-v1",&values)?,"continued":bound.charts().iter().map(|a|canonical(a,&renames)).collect::<Vec<_>>(),"functions":functions,"helpers":semantic_helpers,"common_domain":common});
        if paired {
            semantic_projection["coefficient_zeros"] = serde_json::json!(
                staged
                    .iter()
                    .map(|s| &s.coefficient_zeros)
                    .collect::<Vec<_>>()
            );
            semantic_projection["face_reductions"] = serde_json::json!(
                bound
                    .face_reductions()
                    .iter()
                    .map(|r| (
                        r.chart,
                        r.definition,
                        r.role,
                        canonical(&r.body, &renames),
                        canonical(&r.equation, &renames),
                        canonical(&r.quotient, &renames),
                        canonical(&r.remainder, &renames)
                    ))
                    .collect::<Vec<_>>()
            );
        }
        let runtime_chamber = chamber
            .map(
                |chamber| -> Result<m::UniformParameterChamberV1, KernelError> {
                    let guard = chamber.guard();
                    let guard_semantic = m::Digest(guard.semantic_identity().map_err(invalid)?);
                    let family_certificate = digest(
                        "threshold-uniform-family-v1",
                        &(
                            &source,
                            &guard_semantic,
                            &values,
                            &functions,
                            &semantic_helpers,
                            &common,
                            &policy,
                        ),
                    )?;
                    let descriptor = m::UniformParameterChamberV1 {
                        version: 1,
                        runtime_parameters: runtime_ids,
                        prefix_to_public: guard.prefix_to_public().to_vec(),
                        integration_axes: integration_axes.clone(),
                        predicates: guard.predicate_roles().map_err(invalid)?,
                        guard_semantic,
                        family_certificate,
                    };
                    semantic_projection["projection_version"] = serde_json::json!(5);
                    // Native role-table indices are structural only; the semantic guard
                    // already includes ordered physical names and actual predicates.
                    semantic_projection["uniform_parameter_chamber"] = serde_json::json!({
                        "version": 1, "guard": descriptor.guard_semantic,
                        "family": descriptor.family_certificate,
                        "prefix_to_public": descriptor.prefix_to_public,
                        "integration_axes": descriptor.integration_axes,
                    });
                    Ok(descriptor)
                },
            )
            .transpose()?;
        if matches!(
            request.domain().origin(),
            DomainOrigin::AffineProjective { .. }
        ) {
            semantic_projection["projection_version"] = serde_json::json!(6);
            semantic_projection["source_patch"] = serde_json::json!({
                "origin": format!("{:?}", request.domain().origin()),
                "coordinates": prepared_symbols.iter().map(|s| canonical(&Atom::var(*s), &renames)).collect::<Vec<_>>(),
                "images": source_patch.images.iter().map(|a| canonical(a, &renames)).collect::<Vec<_>>(),
                "measure": canonical(&source_patch.positive_measure, &renames),
                "domain": request.domain().strict_positive().iter().map(|a| canonical(a, &renames)).collect::<Vec<_>>(),
            });
        }
        let semantic = serde_json::to_string(&semantic_projection)?;
        let manifest = m::LineageManifestV1 {
            version: 1,
            preparation: m::Preparation {
                runtime_chamber,
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
                        "threshold-secant-fixed-fiber-v1",
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
            cells,
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
        let mut owner = Self::issued(manifest, atoms, symbols, semantic, orders)?;
        if let Some(chamber) = chamber {
            owner.attach_parameter_guard(chamber.guard().clone())?;
        }
        Ok(owner)
    }
}

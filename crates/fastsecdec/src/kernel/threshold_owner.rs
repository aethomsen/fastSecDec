//! Immutable executable threshold provenance. Constructors are crate-private;
//! plain lineage serde never grants native continuation authority.
use super::threshold_metadata as m;
use super::{KernelError, KernelSet};
use std::{collections::BTreeMap, sync::Arc};
use symbolica::{
    atom::{Atom, Symbol},
    domains::rational::Rational,
};

#[derive(Clone)]
pub struct ThresholdMetadata {
    manifest: Arc<m::LineageManifestV1>,
    atoms: Vec<Atom>,
    local: Option<local::LocalTables>,
    symbols: Vec<Symbol>,
    records: Vec<m::RecordLineageV1>,
    resident: m::ResidentLineageV1,
    /// Compact canonical semantic projection/digests, excludes transport IDs
    /// and grouping. Global source/proof replay requires external evidence.
    semantic: Arc<str>,
    orders: Arc<Vec<i32>>,
    parameter_guard: Option<Arc<super::parameter_guard::ExactParameterGuard>>,
}
#[derive(bincode::Encode, bincode::Decode)]
#[bincode(decode_context = "symbolica::state::StateMap")]
pub(crate) struct Saved {
    descriptor: Vec<u8>,
    atoms: Vec<Atom>,
    symbols: Vec<Symbol>,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Descriptor {
    manifest: m::LineageManifestV1,
    records: Vec<m::RecordLineageV1>,
    resident: m::ResidentLineageV1,
    semantic: String,
    orders: Vec<i32>,
}
fn invalid(e: impl std::fmt::Display) -> KernelError {
    KernelError::Artifact(format!("threshold metadata: {e}"))
}
impl ThresholdMetadata {
    pub(crate) fn combined_record_lineage(
        &self,
    ) -> Result<Option<m::RecordLineageV1>, KernelError> {
        let Some(first) = self.records.first() else {
            return Ok(None);
        };
        if self.records.iter().any(|r| r.kind != first.kind) {
            return Err(invalid("mixed scientific record kinds"));
        }
        let mut contributions = self
            .records
            .iter()
            .flat_map(|r| r.contributions.iter().copied())
            .collect::<Vec<_>>();
        contributions.sort();
        if contributions.windows(2).any(|p| p[0] == p[1]) {
            return Err(invalid("duplicate native contributions"));
        }
        Ok(Some(m::RecordLineageV1 {
            manifest: self.resident.manifest.clone(),
            contributions,
            kind: first.kind.clone(),
        }))
    }
    pub fn lineage(&self) -> &m::LineageManifestV1 {
        &self.manifest
    }
    pub fn result_scope(&self) -> ThresholdResultScope {
        ThresholdResultScope {
            source_extent: self.manifest.preparation.extent.clone(),
            selection: self.resident.selection.clone(),
        }
    }
    pub fn resident(&self) -> &m::ResidentLineageV1 {
        &self.resident
    }
    /// Scientific record associations retained by this resident owner.
    /// The parent lineage may describe additional, unloaded contributions.
    pub fn record_lineage(&self) -> &[m::RecordLineageV1] {
        &self.records
    }
    /// Resolve a role in the saved native symbol table without parsing a name.
    /// Symbols are compact parent metadata, including roles of unloaded charts.
    pub fn native_symbol(&self, id: m::NativeSymbolId) -> Result<Symbol, KernelError> {
        self.symbols
            .get(id.0)
            .copied()
            .ok_or_else(|| invalid("native symbol index"))
    }
    pub fn coefficient_orders(&self) -> &[i32] {
        &self.orders
    }
    pub fn full_original_scope(&self) -> Result<bool, KernelError> {
        self.manifest
            .validate_structure(&self.tables(), m::Limits::default())
            .map_err(invalid)?
            .validate_resident_records(
                &self.manifest.descriptor_digest().map_err(invalid)?,
                &self.resident,
                &self.records,
            )
            .map_err(invalid)
    }
    /// Local saved programs do not contain the global CAD/continuation evidence.
    /// Digest/structural load validation cannot satisfy global proof replay.
    pub fn require_global_proof(&self) -> Result<(), m::Error> {
        Err(m::Error::ProofUnavailable)
    }
    pub(crate) fn tables(&self) -> m::NativeTables {
        if let Some(local) = &self.local {
            return local.tables(self.symbols.len());
        }
        m::NativeTables {
            expression_records: BTreeMap::from([(
                self.manifest.preparation.expressions.clone(),
                self.atoms.len(),
            )]),
            symbols: self.symbols.len(),
        }
    }
    pub(crate) fn validate_layout(
        &self,
        coordinates: &[Vec<Symbol>],
        orders: &[i32],
        runtime: &[Symbol],
    ) -> Result<(), KernelError> {
        self.validate_local_tables()?;
        if self.orders.as_slice() != orders {
            return Err(invalid("fixed fiber/layout differs"));
        }
        self.validate_parameter_guard(runtime)?;
        let checked = self
            .manifest
            .validate_structure(&self.tables(), m::Limits::default())
            .map_err(invalid)?;
        self.manifest
            .validate_executable_schema_kinds()
            .map_err(invalid)?;
        checked
            .validate_resident_records(
                &self.manifest.descriptor_digest().map_err(invalid)?,
                &self.resident,
                &self.records,
            )
            .map_err(invalid)?;
        let actual = self
            .records
            .iter()
            .filter_map(|r| match &r.kind {
                m::RecordKind::Stochastic { coordinates } => Some(
                    coordinates
                        .iter()
                        .map(|i| self.symbols[i.0])
                        .collect::<Vec<_>>(),
                ),
                _ => None,
            })
            .collect::<Vec<_>>();
        if actual != coordinates {
            return Err(invalid("native kernel/lineage coordinate association"));
        }
        for binding in &self.manifest.preparation.fiber.bindings {
            Rational::try_from(
                self.native_atom(&self.manifest.preparation.expressions, binding.value)?
                    .as_view(),
            )
            .map_err(invalid)?;
        }
        if self
            .symbols
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != self.symbols.len()
        {
            return Err(invalid("duplicate native role symbol"));
        }
        let _: serde_json::Value = serde_json::from_str(&self.semantic).map_err(invalid)?;
        Ok(())
    }
    pub(crate) fn validate_algebraic_helpers(
        &self,
        roots: &super::algebraic::Scope,
    ) -> Result<(), KernelError> {
        let resident = self
            .records
            .iter()
            .flat_map(|r| r.contributions.iter())
            .collect::<std::collections::BTreeSet<_>>();
        let charts = self
            .manifest
            .contributions
            .iter()
            .filter(|c| resident.contains(&c.id))
            .map(|c| c.chart)
            .collect::<std::collections::BTreeSet<_>>();
        let admitted = self
            .manifest
            .endpoint_charts
            .iter()
            .filter(|c| charts.contains(&c.id))
            .filter_map(|c| match &c.map {
                m::MapDescriptor::RegularSecantV1 { endpoint, .. } => Some(&endpoint.root_helpers),
                _ => None,
            })
            .flatten()
            .collect::<std::collections::BTreeSet<_>>();
        for (_, bytes) in roots.saved() {
            if !admitted.contains(&m::Digest(blake3::hash(&bytes).to_hex().to_string())) {
                return Err(invalid(
                    "root helper differs from resident endpoint/branch association",
                ));
            }
        }
        Ok(())
    }
    pub(crate) fn visit_atoms(&self, f: &mut impl FnMut(&Atom)) {
        for a in &self.atoms {
            f(a)
        }
        if let Some(local) = &self.local {
            local.visit_atoms(f);
        }
        if let Some(guard) = &self.parameter_guard {
            guard.stored().visit_atoms(f);
            for symbol in guard.stored().symbols() {
                f(&Atom::var(symbol));
            }
        }
        for s in &self.symbols {
            f(&Atom::var(*s))
        }
    }
    pub(crate) fn semantic_identity(
        &self,
        compiler_policy: &str,
        precision: &super::PrecisionPolicy,
    ) -> Result<String, KernelError> {
        self.semantic_identity_for_selection(compiler_policy, precision, &self.resident.selection)
    }
    /// Hash a proposed scope without issuing residency or loading missing records.
    /// Callers must separately establish catalogue/parent and carrier association.
    pub(crate) fn semantic_identity_for_selection(
        &self,
        compiler_policy: &str,
        precision: &super::PrecisionPolicy,
        selection: &m::ResidentSelection,
    ) -> Result<String, KernelError> {
        let mut h = blake3::Hasher::new();
        h.update(b"fastsecdec-threshold-native-v15\0");
        h.update(compiler_policy.as_bytes());
        serde_json::to_writer(&mut h, precision)?;
        h.update(self.semantic.as_bytes());
        serde_json::to_writer(&mut h, self.orders.as_ref())?;
        // Scope is semantic; physical record grouping/transport hashes are not.
        serde_json::to_writer(&mut h, selection)?;
        Ok(h.finalize().to_hex().to_string())
    }
    pub(crate) fn save(&self) -> Result<Saved, KernelError> {
        if self.local.is_some() {
            return Err(invalid("local threshold owner requires native v16"));
        }
        Ok(Saved {
            descriptor: serde_json::to_vec(&Descriptor {
                manifest: self.manifest.as_ref().clone(),
                records: self.records.clone(),
                resident: self.resident.clone(),
                semantic: self.semantic.to_string(),
                orders: self.orders.as_ref().clone(),
            })?,
            atoms: self.atoms.clone(),
            symbols: self.symbols.clone(),
        })
    }
    pub(crate) fn restore(saved: Saved) -> Result<Self, KernelError> {
        let d: Descriptor = serde_json::from_slice(&saved.descriptor)?;
        Ok(Self {
            parameter_guard: None,
            manifest: Arc::new(d.manifest),
            records: d.records,
            resident: d.resident,
            semantic: Arc::from(d.semantic),
            orders: Arc::new(d.orders),
            atoms: saved.atoms,
            local: None,
            symbols: saved.symbols,
        })
    }
}
impl KernelSet {
    pub fn threshold_metadata(&self) -> Option<&ThresholdMetadata> {
        self.threshold.as_deref()
    }
    pub(crate) fn attach_threshold(
        &mut self,
        metadata: ThresholdMetadata,
    ) -> Result<(), KernelError> {
        if self.metadata.is_some()
            || self.program_descriptor.is_some()
            || !self.contour_checks.is_empty()
        {
            return Err(invalid("mixed legacy/threshold provenance"));
        }
        metadata.validate_layout(
            &self
                .sectors
                .iter()
                .map(|s| s.parameters.clone())
                .collect::<Vec<_>>(),
            &self.coefficient_orders,
            &self.runtime_parameters,
        )?;
        metadata.validate_algebraic_helpers(
            &super::algebraic::Scope::merged(self.sectors.iter().map(|s| &s.algebraic))
                .map_err(invalid)?,
        )?;
        self.threshold = Some(Arc::new(metadata));
        Ok(())
    }
}

#[cfg(feature = "threshold-decomposition")]
mod build {
    use super::*;
    use crate::threshold::{
        gcad::DomainOrigin,
        records::{StagedVector, VectorKind},
        regularization::BoundContinuation,
    };
    use symbolica::atom::AtomCore;
    use symbolica::atom::{AtomView, FunctionBuilder};
    fn digest(domain: &str, value: &impl serde::Serialize) -> Result<m::Digest, KernelError> {
        let mut h = blake3::Hasher::new();
        h.update(domain.as_bytes());
        serde_json::to_writer(&mut h, value)?;
        Ok(m::Digest(h.finalize().to_hex().to_string()))
    }
    fn canonical(expression: &Atom, renames: &BTreeMap<Symbol, Symbol>) -> String {
        expression
            .replace_map_bottom_up(|node, _, out| match node {
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
                        **out = b.finish();
                    }
                }
                _ => {}
            })
            .to_canonical_string()
    }
    impl ThresholdMetadata {
        pub(crate) fn from_bound(
            bound: &BoundContinuation<'_>,
            staged: &[StagedVector],
            orders: Vec<i32>,
        ) -> Result<Self, KernelError> {
            Self::from_bound_projection(bound, staged, orders, true)
        }
        #[cfg(test)]
        pub(crate) fn from_bound_historical_projection(
            bound: &BoundContinuation<'_>,
            staged: &[StagedVector],
            orders: Vec<i32>,
        ) -> Result<Self, KernelError> {
            // Frozen v15 semantic projection, used only to construct an old
            // owner wire fixture. Restoring it must never migrate its identity.
            Self::from_bound_projection(bound, staged, orders, false)
        }
        fn from_bound_projection(
            bound: &BoundContinuation<'_>,
            staged: &[StagedVector],
            orders: Vec<i32>,
            compact: bool,
        ) -> Result<Self, KernelError> {
            let certificate = bound.certificate();
            let request = certificate.decomposition().request();
            if request.domain().coordinates().len() != 1
                || staged.len() != certificate.charts().len()
            {
                return Err(invalid("unsupported or incomplete bound chart family"));
            }
            match request.domain().origin() {
                DomainOrigin::NativeUnitCube if request.input().parameters().len() == 1 => {}
                DomainOrigin::AffineProjective { .. }
                    if request.input().parameters().len() == 2 => {}
                _ => {
                    return Err(invalid(
                        "original atlas authority unavailable for this prepared domain",
                    ));
                }
            }
            let mut symbols = request.input().parameters().to_vec();
            symbols.push(request.input().regulator());
            let original = (0..request.input().parameters().len())
                .map(m::NativeSymbolId)
                .collect::<Vec<_>>();
            let regulator = m::NativeSymbolId(symbols.len() - 1);
            let mut physical = request.kinematics().runtime_parameters.clone();
            for s in request.kinematics().exact_values.keys() {
                if !physical.contains(s) {
                    physical.push(*s)
                }
            }
            let physical_ids = physical
                .iter()
                .map(|s| {
                    let i = symbols.len();
                    symbols.push(*s);
                    m::NativeSymbolId(i)
                })
                .collect::<Vec<_>>();
            let unit = m::NativeSymbolId(symbols.len());
            symbols.push(bound.coordinates()[0]);
            let source_patch = super::source_patch::SourcePatch::from_request(request)?;
            let mut atoms = Vec::new();
            let mut push = |a: Atom| {
                let i = atoms.len();
                atoms.push(a);
                m::NativeAtomId(i)
            };
            let one = push(source_patch.positive_measure.clone());
            let mut bindings = Vec::new();
            for (id, s) in physical_ids.iter().zip(&physical) {
                let value = request
                    .kinematics()
                    .exact_values
                    .get(s)
                    .or_else(|| certificate.parameters().get(s))
                    .ok_or_else(|| invalid("incomplete physical fiber"))?;
                bindings.push(m::RationalBinding {
                    symbol: *id,
                    value: push(Atom::num(value.clone())),
                });
            }
            let patch_images = source_patch.images;
            let patch_ids = patch_images
                .iter()
                .cloned()
                .map(&mut push)
                .collect::<Vec<_>>();
            let mut chart_ids = Vec::new();
            for chart in certificate.charts() {
                chart_ids.push((
                    push(chart.image().clone()),
                    push(chart.positive_measure().clone()),
                ));
            }
            let source = m::Digest(request.source_identity().map_err(invalid)?);
            let mut renames = BTreeMap::new();
            renames.insert(
                bound.coordinates()[0],
                symbolica::symbol!("fastsecdec::threshold_semantic::unit_0"),
            );
            // The certified bridge has one native numerator head. Tagged body
            // slots follow source-term/coefficient order; derivatives retain the
            // native DERIVATIVE head. Neither role is inferred from Symbol IDs.
            let mut ordinary = bound
                .definitions()
                .iter()
                .filter(|d| d.derivative_order().is_none());
            let numerator = ordinary
                .next()
                .ok_or_else(|| invalid("missing numerator role"))?
                .head();
            if ordinary.any(|d| d.head() != numerator) {
                return Err(invalid("multiple unassociated numerator roles"));
            }
            renames.insert(
                numerator,
                symbolica::symbol!("fastsecdec::threshold_semantic::numerator"),
            );
            let mut definitions = BTreeMap::new();
            for d in bound.definitions() {
                let role = match d.derivative_order() {
                    None if d.head() == numerator && d.tags().len() == 1 => {
                        (usize::try_from(d.tags()[0].as_view()).map_err(invalid)?, 0)
                    }
                    Some(order)
                        if d.head() == Symbol::DERIVATIVE
                            && d.tags().len() == 4
                            && d.tags()[0].is_zero()
                            && d.tags()[1] == Atom::num(order)
                            && d.tags()[2] == Atom::var(numerator) =>
                    {
                        (
                            usize::try_from(d.tags()[3].as_view()).map_err(invalid)?,
                            order,
                        )
                    }
                    _ => return Err(invalid("unassociated numerator/derivative definition")),
                };
                if definitions.insert(role, d).is_some() {
                    return Err(invalid("duplicate native definition role"));
                }
            }
            let definitions = definitions.into_values().collect::<Vec<_>>();
            let canonical_values = atoms
                .iter()
                .map(|a| canonical(a, &renames))
                .collect::<Vec<_>>();
            let expressions = digest("threshold-native-expression-values-v1", &canonical_values)?;
            let raw = digest(
                "threshold-native-solve-evidence-v1",
                certificate.decomposition().native_result(),
            )?;
            let problem = digest("threshold-native-problem-v1", request.problem())?;
            let continuation = digest(
                "threshold-epsilon-strip-v1",
                &(
                    certificate
                        .convergence_strip()
                        .lower()
                        .map(ToString::to_string),
                    certificate
                        .convergence_strip()
                        .upper()
                        .map(ToString::to_string),
                    &orders,
                ),
            )?;
            let coverage = digest(
                "threshold-interval-partition-v1",
                &certificate
                    .charts()
                    .iter()
                    .map(|c| {
                        (
                            c.interval().0.to_string(),
                            c.interval().1.to_string(),
                            c.orientation(),
                        )
                    })
                    .collect::<Vec<_>>(),
            )?;
            let admission = digest(
                "threshold-fixed-fiber-v1",
                &physical
                    .iter()
                    .zip(&bindings)
                    .map(|(s, b)| {
                        (
                            Atom::var(*s).to_canonical_string(),
                            canonical_values[b.value.0].clone(),
                        )
                    })
                    .collect::<Vec<_>>(),
            )?;
            let patch_coordinates = request
                .domain()
                .coordinates()
                .iter()
                .map(|s| m::NativeSymbolId(symbols.iter().position(|t| t == s).unwrap()))
                .collect::<Vec<_>>();
            let patch = m::Patch {
                id: m::PatchId(0),
                source: m::SourceOrigin::OriginalInput,
                map: m::MapDescriptor::RationalV1 {
                    geometry: m::MapGeometry {
                        expressions: expressions.clone(),
                        coordinates: patch_coordinates.clone(),
                        images: patch_ids,
                        positive_measure: one,
                        orientation: m::Orientation::Forward,
                        certificate: coverage.clone(),
                    },
                },
                coverage: coverage.clone(),
            };
            let options = bound.generation_options();
            let continuation_policy = serde_json::json!({"mode":options.mode,"subtraction":options.subtraction,"max_order":options.max_order,"max_subtractions_per_axis":options.max_subtractions_per_axis,"max_subtraction_terms":options.max_subtraction_terms,"coefficient_expansion":options.coefficient_expansion});
            let numerator_closure = definitions
                .iter()
                .map(|d| {
                    digest(
                        "threshold-canonical-function-v1",
                        &(
                            d.derivative_order(),
                            d.tags()
                                .iter()
                                .map(|a| canonical(a, &renames))
                                .collect::<Vec<_>>(),
                            canonical(d.body(), &renames),
                        ),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            let mut cells = BTreeMap::new();
            let mut endpoint_charts = Vec::new();
            let mut contributions = Vec::new();
            for (index, (chart, staged)) in certificate.charts().iter().zip(staged).enumerate() {
                if staged.chart != index {
                    return Err(invalid("staged chart order"));
                }
                let raw_index = chart.admission().map().source().cell_index();
                cells.entry(raw_index).or_insert(m::Cell {
                    id: m::CellId(raw_index),
                    patch: m::PatchId(0),
                    native: m::NativeCellLocator {
                        request_record: problem.clone(),
                        raw_evidence: raw.clone(),
                        raw_cell_index: raw_index,
                    },
                    coordinate_order: patch_coordinates.clone(),
                    coverage: coverage.clone(),
                });
                endpoint_charts.push(m::EndpointChart{id:m::EndpointChartId(index),origin:m::EndpointOrigin::Cell(m::CellId(raw_index)),path:vec![index],map:m::MapDescriptor::RationalV1{geometry:m::MapGeometry {
                    expressions:expressions.clone(),coordinates:vec![unit],images:vec![chart_ids[index].0],positive_measure:chart_ids[index].1,orientation:if chart.orientation()>0{m::Orientation::Forward}else{m::Orientation::Reversed},certificate:coverage.clone(),
                }},endpoint_certificate:digest("threshold-native-closed-endpoint-v1",&serde_json::json!({"policy":continuation_policy,"strip":continuation,"numerator_closure":numerator_closure,"regular_epsilon":certificate.prefactor_witness().rational_witness().epsilon().to_string(),"gamma_witness":certificate.prefactor_witness().gamma_witnesses().iter().map(|g|(g.term(),g.factor(),g.argument_value().to_string(),g.pole_distance().map(ToString::to_string))).collect::<Vec<_>>(),"terms":(0..request.prepared_terms().len()).map(|term| {
                    let (prefactor,regular,power)=chart.normalized_term(term).expect("certified complete term");
                    serde_json::json!({"prefactor":canonical(prefactor,&renames),"regular":canonical(regular,&renames),"power":canonical(power,&renames),"units":chart.units(term).expect("certified units").iter().map(|u|serde_json::json!({"factor":u.factor_index(),"multiplicity":u.multiplicity(),"residual":canonical(u.residual(),&renames),"endpoints":[u.endpoint_values().0.to_string(),u.endpoint_values().1.to_string()]})).collect::<Vec<_>>()})
                }).collect::<Vec<_>>()}))?,causal_phase_certificate:digest("threshold-bound-phase-density-v1",&canonical(&bound.chart_expressions()[index],&renames))?});
                let kind = match staged.kind {
                    VectorKind::Stochastic {} => m::ContributionKind::Stochastic {
                        coordinates: vec![unit],
                    },
                    VectorKind::Exact {} => m::ContributionKind::Exact,
                    VectorKind::ZeroInLayout {} => m::ContributionKind::CertifiedZero {
                        certificate: digest(
                            "threshold-zero-in-requested-layout-v1",
                            &(&source, index, &orders),
                        )?,
                    },
                };
                contributions.push(m::Contribution {
                    id: m::ContributionId(index),
                    chart: m::EndpointChartId(index),
                    group: m::GroupId(0),
                    kind,
                });
            }
            let mut projection = serde_json::json!({"source":source,"continuation_policy":continuation_policy,"strategy":"gcad_first","origin":format!("{:?}",request.domain().origin()),"domain":request.domain().strict_positive().iter().map(|a|canonical(a,&renames)).collect::<Vec<_>>(),"parameter_chamber":request.kinematics().strict_positive.iter().map(|a|canonical(a,&renames)).collect::<Vec<_>>(),"fiber":physical.iter().zip(&bindings).map(|(s,b)|(Atom::var(*s).to_canonical_string(),canonical_values[b.value.0].clone())).collect::<Vec<_>>(),"maps":canonical_values,"continued_charts":bound.chart_expressions().iter().map(|a|digest("threshold-canonical-continued-chart-v1",&canonical(a,&renames))).collect::<Result<Vec<_>,_>>()?,"functions":definitions.iter().map(|d|digest("threshold-canonical-function-v1",&(d.derivative_order(),d.tags().iter().map(|a|canonical(a,&renames)).collect::<Vec<_>>(),canonical(d.body(),&renames)))).collect::<Result<Vec<_>,_>>()?,"strip":[certificate.convergence_strip().lower().map(ToString::to_string),certificate.convergence_strip().upper().map(ToString::to_string)]});
            if compact {
                let object = projection.as_object_mut().expect("native semantic object");
                for (field, domain) in [
                    ("maps", "threshold-canonical-map-list-v2"),
                    ("domain", "threshold-canonical-domain-list-v2"),
                    ("parameter_chamber", "threshold-canonical-chamber-list-v2"),
                ] {
                    let value = object.remove(field).expect("native semantic field");
                    object.insert(
                        format!("{field}_digest"),
                        serde_json::to_value(digest(domain, &value)?)?,
                    );
                }
                object.insert("projection_version".into(), serde_json::json!(2));
            }
            let semantic = serde_json::to_string(&projection)?;
            let manifest = m::LineageManifestV1 {
                version: 1,
                preparation: m::Preparation {
                    runtime_chamber: None,
                    source_identity: source,
                    extent: m::SourceExtent::Full {
                        ordinary_geometry: None,
                    },
                    strategy: m::ResolvedStrategy::GcadFirst,
                    actual_options: digest(
                        "threshold-continuation-policy-v1",
                        &continuation_policy,
                    )?,
                    expressions,
                    partition_certificate: coverage,
                    original_coordinates: original,
                    regulator,
                    fiber: m::FixedRationalFiber {
                        physical_parameters: physical_ids,
                        bindings,
                        admission,
                    },
                },
                patches: vec![patch],
                cells: cells.into_values().collect(),
                endpoint_charts,
                continuation_groups: vec![m::ContinuationGroup {
                    id: m::GroupId(0),
                    charts: (0..staged.len()).map(m::EndpointChartId).collect(),
                    continuation: m::Continuation::EpsilonStripV1 {
                        certificate: continuation,
                    },
                }],
                contributions,
            };
            Self::issued(manifest, atoms, symbols, semantic, orders)
        }
    }
}

/// Inspection-only scope declaration retained by generic result manifests.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThresholdResultScope {
    source_extent: m::SourceExtent,
    selection: m::ResidentSelection,
}
impl ThresholdResultScope {
    pub(crate) fn complete_catalogue(source_extent: m::SourceExtent) -> Result<Self, KernelError> {
        let scope = Self {
            source_extent,
            selection: m::ResidentSelection::Complete,
        };
        scope.validate()?;
        Ok(scope)
    }
    pub fn is_full_original(&self) -> bool {
        matches!(self.source_extent, m::SourceExtent::Full { .. })
            && matches!(self.selection, m::ResidentSelection::Complete)
    }
    pub fn source_extent(&self) -> &m::SourceExtent {
        &self.source_extent
    }
    pub fn selection(&self) -> &m::ResidentSelection {
        &self.selection
    }
    pub(crate) fn validate(&self) -> Result<(), KernelError> {
        if let m::SourceExtent::Selected { selection, .. } = &self.source_extent {
            selection.validate().map_err(invalid)?;
        }
        Ok(())
    }
}

mod local;
mod parameters;
pub(crate) use local::SavedV16;
#[cfg(feature = "threshold-decomposition")]
pub(crate) use local::{StagedTablePlan, StagedTables};

#[cfg(feature = "threshold-decomposition")]
mod algebraic;

#[cfg(feature = "threshold-decomposition")]
impl ThresholdMetadata {
    fn issued(
        manifest: m::LineageManifestV1,
        atoms: Vec<Atom>,
        symbols: Vec<Symbol>,
        semantic: String,
        orders: Vec<i32>,
    ) -> Result<Self, KernelError> {
        let descriptor = manifest.descriptor_digest().map_err(invalid)?;
        let records = manifest
            .contributions
            .iter()
            .filter_map(|c| {
                let kind = match &c.kind {
                    m::ContributionKind::Stochastic { coordinates } => m::RecordKind::Stochastic {
                        coordinates: coordinates.clone(),
                    },
                    m::ContributionKind::Exact => m::RecordKind::Exact,
                    m::ContributionKind::CertifiedZero { .. } => return None,
                };
                Some(m::RecordLineageV1 {
                    manifest: descriptor.clone(),
                    contributions: vec![c.id],
                    kind,
                })
            })
            .collect();
        Ok(Self {
            parameter_guard: None,
            manifest: Arc::new(manifest),
            atoms,
            local: None,
            symbols,
            records,
            resident: m::ResidentLineageV1 {
                manifest: descriptor,
                selection: m::ResidentSelection::Complete,
            },
            semantic: Arc::from(semantic),
            orders: Arc::new(orders),
        })
    }
}

#[cfg(feature = "threshold-decomposition")]
mod source_patch;

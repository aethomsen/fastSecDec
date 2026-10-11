use super::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    Invalid(String),
    Limit,
    Unsupported(String),
    ProofUnavailable,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(s) => write!(f, "threshold lineage: {s}"),
            Self::Limit => write!(f, "threshold lineage structural limit"),
            Self::Unsupported(s) => write!(f, "unsupported threshold capability: {s}"),
            Self::ProofUnavailable => write!(f, "threshold global proof evidence unavailable"),
        }
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;
/// Immutable structural-admission view. It certifies no mathematics and cannot
/// be deserialized. The borrow prevents later mutations bypassing its checks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructureCheckedLineage<'a> {
    pub(super) manifest: &'a LineageManifestV1,
    pub(super) descriptor: Digest,
}
pub(super) fn invalid(s: impl Into<String>) -> Error {
    Error::Invalid(s.into())
}

/// Post-decode structural bounds, not an allocator or process RSS limit.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub maximum_nodes: usize,
    pub maximum_references: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            maximum_nodes: 1_000_000,
            maximum_references: 10_000_000,
        }
    }
}
struct Budget {
    nodes: usize,
    references: usize,
    limits: Limits,
}
impl Budget {
    fn nodes(&mut self, n: usize) -> Result<()> {
        self.nodes = self.nodes.checked_add(n).ok_or(Error::Limit)?;
        if self.nodes > self.limits.maximum_nodes {
            return Err(Error::Limit);
        }
        Ok(())
    }
    fn refs(&mut self, n: usize) -> Result<()> {
        self.references = self.references.checked_add(n).ok_or(Error::Limit)?;
        if self.references > self.limits.maximum_references {
            return Err(Error::Limit);
        }
        Ok(())
    }
}
impl Digest {
    pub fn validate(&self) -> Result<()> {
        if self.0.len() != 64
            || !self
                .0
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid("invalid lowercase content digest"));
        }
        Ok(())
    }
}
fn unique<T: Ord + Copy>(values: &[T], name: &str) -> Result<BTreeSet<T>> {
    let set = values.iter().copied().collect::<BTreeSet<_>>();
    if set.len() != values.len() {
        return Err(invalid(format!("duplicate {name}")));
    }
    Ok(set)
}
pub(super) fn ordered<T: Ord>(values: &[T], name: &str) -> Result<()> {
    if values.windows(2).any(|p| p[0] >= p[1]) {
        return Err(invalid(format!("noncanonical or duplicate {name}")));
    }
    Ok(())
}
fn symbol(id: NativeSymbolId, tables: &NativeTables) -> Result<()> {
    if id.0 >= tables.symbols {
        return Err(invalid("native symbol reference outside payload"));
    }
    Ok(())
}
fn atom(id: NativeAtomId, record: &Digest, tables: &NativeTables) -> Result<()> {
    let count = tables
        .expression_records
        .get(record)
        .ok_or_else(|| invalid("missing native expression record schema"))?;
    if id.0 >= *count {
        return Err(invalid("native Atom reference outside payload"));
    }
    Ok(())
}
fn symbols(ids: &[NativeSymbolId], tables: &NativeTables, budget: &mut Budget) -> Result<()> {
    budget.refs(ids.len())?;
    unique(ids, "coordinate/parameter symbol")?;
    ids.iter().try_for_each(|id| symbol(*id, tables))
}
impl MapDescriptor {
    fn validate(&self, tables: &NativeTables, budget: &mut Budget) -> Result<()> {
        let g = self.geometry();
        g.expressions.validate()?;
        symbols(&g.coordinates, tables, budget)?;
        budget.refs(g.images.len())?;
        g.images
            .iter()
            .try_for_each(|id| atom(*id, &g.expressions, tables))?;
        atom(g.positive_measure, &g.expressions, tables)?;
        g.certificate.validate()?;
        if let Self::AlgebraicSectionsV1 { sections, .. } | Self::RegularSecantV1 { sections, .. } =
            self
        {
            if sections.is_empty() {
                return Err(invalid("algebraic map has no sections"));
            }
            budget.nodes(sections.len())?;
            let mut available = g.coordinates.iter().copied().collect::<BTreeSet<_>>();
            for section in sections {
                atom(section.polynomial, &g.expressions, tables)?;
                symbol(section.root_variable, tables)?;
                symbols(&section.preceding_coordinates, tables, budget)?;
                if section
                    .preceding_coordinates
                    .iter()
                    .any(|v| !available.contains(v))
                    || !available.insert(section.root_variable)
                {
                    return Err(invalid("algebraic section order or root variable"));
                }
                section.branch_certificate.validate()?;
            }
        }
        if let Self::RegularSecantV1 { endpoint, .. } = self {
            symbol(endpoint.regulator, tables)?;
            endpoint.continuation_policy.validate()?;
            endpoint.common_domain.validate()?;
            endpoint.numerator_functions.validate()?;
            budget.nodes(endpoint.terms.len())?;
            budget.refs(endpoint.root_helpers.len())?;
            if endpoint.terms.is_empty() || endpoint.root_helpers.is_empty() {
                return Err(invalid("empty regular-section endpoint record"));
            }
            for d in &endpoint.root_helpers {
                d.validate()?;
            }
            ordered(&endpoint.root_helpers, "root helper identities")?;
            for term in &endpoint.terms {
                if term.powers.len() != g.coordinates.len() {
                    return Err(invalid("endpoint power dimension"));
                }
                budget.refs(term.powers.len() + 2)?;
                atom(term.prefactor, &g.expressions, tables)?;
                atom(term.regular, &g.expressions, tables)?;
                for p in &term.powers {
                    atom(*p, &g.expressions, tables)?;
                }
                budget.nodes(term.factors.len())?;
                for f in &term.factors {
                    if f.source_term != term.source_term {
                        return Err(invalid("factor source-term association"));
                    }
                    atom(f.source, &g.expressions, tables)?;
                    atom(f.exponent, &g.expressions, tables)?;
                    f.certificate.validate()?;
                }
            }
        }
        Ok(())
    }
}

impl LineageManifestV1 {
    /// Validate only data shape and cross-references. Digests and certificate
    /// references are not themselves proofs or native-value admission.
    pub fn validate_structure(
        &self,
        tables: &NativeTables,
        limits: Limits,
    ) -> Result<StructureCheckedLineage<'_>> {
        if self.version != 1 {
            return Err(Error::Unsupported("lineage version".into()));
        }
        let mut budget = Budget {
            nodes: 0,
            references: 0,
            limits,
        };
        for n in [
            self.patches.len(),
            self.cells.len(),
            self.endpoint_charts.len(),
            self.continuation_groups.len(),
            self.contributions.len(),
        ] {
            budget.nodes(n)?;
        }
        let p = &self.preparation;
        p.source_identity.validate()?;
        p.actual_options.validate()?;
        p.expressions.validate()?;
        if !tables.expression_records.contains_key(&p.expressions) {
            return Err(invalid("missing preparation expression record schema"));
        }
        p.partition_certificate.validate()?;
        p.fiber.admission.validate()?;
        symbols(&p.original_coordinates, tables, &mut budget)?;
        symbol(p.regulator, tables)?;
        symbols(&p.fiber.physical_parameters, tables, &mut budget)?;
        if p.original_coordinates.contains(&p.regulator)
            || p.fiber
                .physical_parameters
                .iter()
                .any(|s| *s == p.regulator || p.original_coordinates.contains(s))
            || p.fiber.bindings.iter().map(|b| b.symbol).ne(p
                .fiber
                .physical_parameters
                .iter()
                .copied())
        {
            return Err(invalid("coordinate/regulator/physical binding roles"));
        }
        let runtime = p
            .runtime_chamber
            .as_ref()
            .map_or(&[][..], |c| c.runtime_parameters.as_slice());
        if let Some(chamber) = &p.runtime_chamber {
            let n = chamber.runtime_parameters.len();
            if chamber.version != 1 || n == 0 {
                return Err(invalid("uniform parameter chamber capability"));
            }
            symbols(runtime, tables, &mut budget)?;
            budget.refs(chamber.prefix_to_public.len())?;
            budget.refs(chamber.integration_axes.len())?;
            budget.nodes(chamber.predicates.len())?;
            chamber.guard_semantic.validate()?;
            chamber.family_certificate.validate()?;
            if chamber.prefix_to_public.len() != n
                || chamber
                    .prefix_to_public
                    .iter()
                    .copied()
                    .collect::<BTreeSet<_>>()
                    != (0..n).collect()
                || chamber.integration_axes.len() != p.original_coordinates.len()
                || chamber.integration_axes.iter().copied().ne(n..n
                    .checked_add(p.original_coordinates.len())
                    .ok_or(Error::Limit)?)
                || runtime.iter().any(|s| {
                    *s == p.regulator
                        || p.original_coordinates.contains(s)
                        || p.fiber.physical_parameters.contains(s)
                })
            {
                return Err(invalid("uniform runtime/prefix/integration roles"));
            }
            let mut predicates = BTreeSet::new();
            for row in &chamber.predicates {
                row.predicate.validate()?;
                if !predicates.insert((row.role, row.index)) {
                    return Err(invalid("duplicate parameter predicate role"));
                }
                match row.role {
                    ParameterPredicateRole::LowerBound | ParameterPredicateRole::UpperBound
                        if row.index >= n =>
                    {
                        return Err(invalid("parameter bound axis"));
                    }
                    ParameterPredicateRole::StrictPositive | ParameterPredicateRole::Nonzero
                        if row.positive_selector =>
                    {
                        return Err(invalid("selector on non-bound predicate"));
                    }
                    _ => {}
                }
            }
            for role in [
                ParameterPredicateRole::StrictPositive,
                ParameterPredicateRole::Nonzero,
            ] {
                let indices = predicates
                    .iter()
                    .filter_map(|(r, i)| (*r == role).then_some(*i))
                    .collect::<Vec<_>>();
                if indices.iter().copied().ne(0..indices.len()) {
                    return Err(invalid("parameter predicate inventory"));
                }
            }
        }
        budget.refs(p.fiber.bindings.len())?;
        for binding in &p.fiber.bindings {
            atom(binding.value, &p.expressions, tables)?;
        }
        for map in self
            .patches
            .iter()
            .map(|p| &p.map)
            .chain(self.endpoint_charts.iter().map(|c| &c.map))
        {
            if map.geometry().coordinates.iter().any(|s| {
                *s == p.regulator || p.fiber.physical_parameters.contains(s) || runtime.contains(s)
            }) {
                return Err(invalid(
                    "map coordinate captures a regulator or physical parameter",
                ));
            }
            if let MapDescriptor::RegularSecantV1 { endpoint, .. } = map
                && endpoint.regulator != p.regulator
            {
                return Err(invalid("normalized endpoint regulator differs from source"));
            }
            if let MapDescriptor::AlgebraicSectionsV1 { sections, .. }
            | MapDescriptor::RegularSecantV1 { sections, .. } = map
                && sections.iter().any(|s| {
                    s.root_variable == p.regulator
                        || p.fiber.physical_parameters.contains(&s.root_variable)
                        || runtime.contains(&s.root_variable)
                })
            {
                return Err(invalid(
                    "algebraic root variable captures a regulator or physical parameter",
                ));
            }
        }

        let expected_sources = match (&p.extent, p.strategy) {
            (
                SourceExtent::Full {
                    ordinary_geometry: None,
                },
                ResolvedStrategy::GcadFirst,
            ) => BTreeSet::from([None]),
            (
                SourceExtent::Full {
                    ordinary_geometry: Some(geometry),
                },
                ResolvedStrategy::SectorFirst,
            ) => {
                geometry.identity.validate()?;
                if geometry.original_source_count == 0 {
                    return Err(invalid("empty ordinary geometry"));
                }
                budget.refs(geometry.original_source_count)?;
                (0..geometry.original_source_count).map(Some).collect()
            }
            (
                SourceExtent::Selected {
                    selection,
                    ordinary_geometry,
                },
                ResolvedStrategy::SectorFirst,
            ) => {
                selection.validate().map_err(|e| invalid(e.to_string()))?;
                ordinary_geometry.validate()?;
                budget.refs(selection.source_sectors().len())?;
                selection
                    .source_sectors()
                    .iter()
                    .copied()
                    .map(Some)
                    .collect()
            }
            _ => {
                return Err(invalid(
                    "resolved strategy and original source extent disagree",
                ));
            }
        };
        ordered(
            &self.patches.iter().map(|v| v.id).collect::<Vec<_>>(),
            "patch IDs",
        )?;
        let mut patches = BTreeMap::new();
        let mut observed_sources = BTreeSet::new();
        for patch in &self.patches {
            let source = match patch.source {
                SourceOrigin::OriginalInput => None,
                SourceOrigin::OrdinarySector(i) => Some(i),
            };
            if !expected_sources.contains(&source) {
                return Err(invalid("patch source outside original extent"));
            }
            observed_sources.insert(source);
            patch.coverage.validate()?;
            patch.map.validate(tables, &mut budget)?;
            if patch.map.geometry().images.len() != p.original_coordinates.len() {
                return Err(invalid(
                    "patch images do not cover original coordinate order",
                ));
            }
            patches.insert(patch.id, patch);
        }
        if observed_sources != expected_sources {
            return Err(invalid("selected original source omitted"));
        }
        ordered(
            &self.cells.iter().map(|v| v.id).collect::<Vec<_>>(),
            "cell IDs",
        )?;
        let mut cells = BTreeMap::new();
        let mut native_cells = BTreeSet::new();
        let mut patch_requests = BTreeMap::new();
        let mut used_patches = BTreeSet::new();
        for cell in &self.cells {
            let patch = patches
                .get(&cell.patch)
                .ok_or_else(|| invalid("unknown cell patch"))?;
            if patch.map.geometry().coordinates.is_empty() || cell.coordinate_order.is_empty() {
                return Err(invalid("zero-dimensional preparation has no GCAD cell"));
            }
            used_patches.insert(cell.patch);
            cell.native.request_record.validate()?;
            cell.native.raw_evidence.validate()?;
            cell.coverage.validate()?;
            let owner = (&cell.native.request_record, &cell.native.raw_evidence);
            if patch_requests
                .insert(cell.patch, owner)
                .is_some_and(|previous| previous != owner)
            {
                return Err(invalid("patch mixes distinct request/evidence owners"));
            }
            if !native_cells.insert((
                cell.patch,
                &cell.native.request_record,
                &cell.native.raw_evidence,
                cell.native.raw_cell_index,
            )) {
                return Err(invalid("duplicate raw cell in one patch"));
            }
            symbols(&cell.coordinate_order, tables, &mut budget)?;
            if cell
                .coordinate_order
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                != patch
                    .map
                    .geometry()
                    .coordinates
                    .iter()
                    .copied()
                    .collect::<BTreeSet<_>>()
            {
                return Err(invalid("cell coordinate permutation differs from patch"));
            }
            cells.insert(cell.id, cell);
        }
        ordered(
            &self
                .endpoint_charts
                .iter()
                .map(|v| v.id)
                .collect::<Vec<_>>(),
            "endpoint chart IDs",
        )?;
        let mut charts = BTreeMap::new();
        let mut used_cells = BTreeSet::new();
        let mut paths = BTreeSet::new();
        for chart in &self.endpoint_charts {
            let dimension = match chart.origin {
                EndpointOrigin::Cell(id) => {
                    let cell = cells
                        .get(&id)
                        .ok_or_else(|| invalid("unknown endpoint cell"))?;
                    used_cells.insert(id);
                    cell.coordinate_order.len()
                }
                EndpointOrigin::ExactPatch(id) => {
                    let patch = patches
                        .get(&id)
                        .ok_or_else(|| invalid("unknown exact preparation patch"))?;
                    if !patch.map.geometry().coordinates.is_empty() || used_patches.contains(&id) {
                        return Err(invalid(
                            "exact preparation must be zero-dimensional without GCAD cells",
                        ));
                    }
                    used_patches.insert(id);
                    0
                }
            };
            budget.refs(chart.path.len())?;
            if !paths.insert((chart.origin, &chart.path)) {
                return Err(invalid("duplicate endpoint path"));
            }
            chart.map.validate(tables, &mut budget)?;
            if chart.map.geometry().images.len() != dimension
                || chart.map.geometry().coordinates.len() != dimension
            {
                return Err(invalid("endpoint chart map dimension"));
            }
            chart.endpoint_certificate.validate()?;
            chart.causal_phase_certificate.validate()?;
            charts.insert(chart.id, chart);
        }
        if used_patches.len() != patches.len() {
            return Err(invalid("patch lacks coverage descendants"));
        }
        if used_cells.len() != cells.len() {
            return Err(invalid("cell lacks endpoint descendants"));
        }
        ordered(
            &self
                .continuation_groups
                .iter()
                .map(|v| v.id)
                .collect::<Vec<_>>(),
            "continuation IDs",
        )?;
        let mut memberships = BTreeMap::new();
        for group in &self.continuation_groups {
            if group.charts.is_empty() {
                return Err(invalid("empty continuation group"));
            }
            budget.refs(group.charts.len())?;
            ordered(&group.charts, "continuation chart list")?;
            group.continuation.certificate().validate()?;
            for chart in &group.charts {
                if !charts.contains_key(chart) || memberships.insert(*chart, group.id).is_some() {
                    return Err(invalid("unknown or multiply continued endpoint chart"));
                }
            }
        }
        if memberships.len() != charts.len() {
            return Err(invalid("endpoint chart lacks continuation"));
        }
        ordered(
            &self.contributions.iter().map(|v| v.id).collect::<Vec<_>>(),
            "contribution IDs",
        )?;
        let mut used_charts = BTreeSet::new();
        for contribution in &self.contributions {
            let chart = charts
                .get(&contribution.chart)
                .ok_or_else(|| invalid("unknown contribution chart"))?;
            if memberships.get(&contribution.chart) != Some(&contribution.group) {
                return Err(invalid("contribution outside its continuation group"));
            }
            used_charts.insert(contribution.chart);
            match &contribution.kind {
                ContributionKind::Stochastic { coordinates } => {
                    symbols(coordinates, tables, &mut budget)?;
                    if coordinates.is_empty()
                        || coordinates
                            .iter()
                            .any(|s| !chart.map.geometry().coordinates.contains(s))
                    {
                        return Err(invalid("stochastic contribution dimension"));
                    }
                }
                ContributionKind::Exact => {}
                ContributionKind::CertifiedZero { certificate } => certificate.validate()?,
            }
        }
        if used_charts.len() != charts.len() {
            return Err(invalid("endpoint chart contribution omitted"));
        }
        Ok(StructureCheckedLineage {
            manifest: self,
            descriptor: self.descriptor_digest()?,
        })
    }

    /// A schema-kind fence, not proof or dimension admission. The actual first
    /// factory separately requires a checked zero/one-dimensional completion.
    pub fn validate_initial_schema_kinds(&self) -> Result<()> {
        if self
            .patches
            .iter()
            .map(|v| &v.map)
            .chain(self.endpoint_charts.iter().map(|v| &v.map))
            .any(|m| !matches!(m, MapDescriptor::RationalV1 { .. }))
        {
            return Err(Error::Unsupported("algebraic moving sections".into()));
        }
        if self
            .continuation_groups
            .iter()
            .any(|g| !matches!(g.continuation, Continuation::EpsilonStripV1 { .. }))
        {
            return Err(Error::Unsupported("auxiliary integral cancellation".into()));
        }
        Ok(())
    }

    /// Arithmetic schema fence for the sealed native factories. This accepts
    /// only the issued fixed regular-section successor, never the generic future
    /// algebraic/auxiliary descriptors. It is still not proof revalidation.
    pub(crate) fn validate_executable_schema_kinds(&self) -> Result<()> {
        if self
            .patches
            .iter()
            .map(|p| &p.map)
            .chain(self.endpoint_charts.iter().map(|p| &p.map))
            .any(|m| {
                !matches!(
                    m,
                    MapDescriptor::RationalV1 { .. } | MapDescriptor::RegularSecantV1 { .. }
                )
            })
        {
            return Err(Error::Unsupported("unissued algebraic map recipe".into()));
        }
        if self
            .continuation_groups
            .iter()
            .any(|g| !matches!(g.continuation, Continuation::EpsilonStripV1 { .. }))
        {
            return Err(Error::Unsupported("auxiliary integral cancellation".into()));
        }
        Ok(())
    }

    /// Canonical structural descriptor digest. Native Atom/Symbol values are
    /// absent. Mathematical identity needs a separate canonical semantic
    /// projection that excludes transport digests, native State registration
    /// order and record grouping. Adding canonical native values to this hash
    /// cannot remove those dependencies. This digest only binds descriptors.
    pub fn descriptor_digest(&self) -> Result<Digest> {
        let mut hash = blake3::Hasher::new();
        hash.update(b"fastsecdec-threshold-lineage-descriptor-v1\0");
        serde_json::to_writer(&mut hash, self).map_err(|e| invalid(e.to_string()))?;
        Ok(Digest(hash.finalize().to_hex().to_string()))
    }
}

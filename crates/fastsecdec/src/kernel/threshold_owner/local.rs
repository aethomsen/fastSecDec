//! Record-local native tables. The directory is structural evidence; native
//! values are imported only for the resident contributions and their ancestors.
use super::*;
use std::collections::BTreeSet;
use symbolica::atom::AtomCore;

#[derive(Clone)]
pub(super) struct LocalTables {
    directory: Arc<BTreeMap<m::Digest, usize>>,
    values: BTreeMap<m::Digest, Arc<Vec<Atom>>>,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Parent {
    manifest: m::LineageManifestV1,
    directory: BTreeMap<m::Digest, usize>,
    semantic: String,
    orders: Vec<i32>,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Residency {
    records: Vec<m::RecordLineageV1>,
    resident: m::ResidentLineageV1,
}
/// Explicit successor envelope. v15's flat native table remains unchanged.
#[derive(bincode::Encode, bincode::Decode)]
#[bincode(decode_context = "symbolica::state::StateMap")]
pub(crate) struct SavedV16 {
    parent: Vec<u8>,
    parent_id: String,
    residency: Vec<u8>,
    values: Vec<(String, Vec<Atom>)>,
    symbols: Vec<Symbol>,
}
impl LocalTables {
    pub(super) fn tables(&self, symbols: usize) -> m::NativeTables {
        m::NativeTables {
            expression_records: self.directory.as_ref().clone(),
            symbols,
        }
    }
    pub(super) fn visit_atoms(&self, f: &mut impl FnMut(&Atom)) {
        for values in self.values.values() {
            for atom in values.iter() {
                f(atom);
            }
        }
    }
}
fn atom_id(
    atoms: &[Atom],
    id: &mut m::NativeAtomId,
    remap: &mut BTreeMap<usize, usize>,
    values: &mut Vec<Atom>,
) -> Result<(), KernelError> {
    let old = id.0;
    id.0 = if let Some(new) = remap.get(&old) {
        *new
    } else {
        let value = atoms
            .get(old)
            .ok_or_else(|| invalid("flat table index"))?
            .clone();
        let new = values.len();
        values.push(value);
        remap.insert(old, new);
        new
    };
    Ok(())
}
fn table_digest(values: &[Atom]) -> m::Digest {
    // This is a structural table identity, never a mathematical program ID.
    let mut hash = blake3::Hasher::new();
    hash.update(b"threshold-native-local-table-v1\0");
    for atom in values {
        let value = atom.to_canonical_string();
        hash.update(&(value.len() as u64).to_le_bytes());
        hash.update(value.as_bytes());
    }
    m::Digest(hash.finalize().to_hex().to_string())
}
fn insert_table(
    values: Vec<Atom>,
    tables: &mut BTreeMap<m::Digest, Arc<Vec<Atom>>>,
) -> Result<m::Digest, KernelError> {
    let digest = table_digest(&values);
    if let Some(old) = tables.get(&digest) {
        if old.as_ref() != &values {
            return Err(invalid("native table digest collision"));
        }
    } else {
        tables.insert(digest.clone(), Arc::new(values));
    }
    Ok(digest)
}
fn localize_map(
    map: &mut m::MapDescriptor,
    atoms: &[Atom],
    tables: &mut BTreeMap<m::Digest, Arc<Vec<Atom>>>,
) -> Result<(), KernelError> {
    let (geometry, sections, endpoint) = match map {
        m::MapDescriptor::RegularSecantV1 {
            geometry,
            sections,
            endpoint,
        } => (geometry, Some(sections), Some(endpoint)),
        m::MapDescriptor::RationalV1 { geometry } => (geometry, None, None),
        m::MapDescriptor::AlgebraicSectionsV1 { geometry, sections } => {
            (geometry, Some(sections), None)
        }
    };
    let mut values = Vec::new();
    let mut remap = BTreeMap::new();
    for image in &mut geometry.images {
        atom_id(atoms, image, &mut remap, &mut values)?;
    }
    atom_id(
        atoms,
        &mut geometry.positive_measure,
        &mut remap,
        &mut values,
    )?;
    if let Some(sections) = sections {
        for section in sections {
            atom_id(atoms, &mut section.polynomial, &mut remap, &mut values)?;
        }
    }
    if let Some(endpoint) = endpoint {
        endpoint.map_atoms(|id| atom_id(atoms, id, &mut remap, &mut values))?;
    }
    geometry.expressions = insert_table(values, tables)?;
    Ok(())
}
impl ThresholdMetadata {
    pub(crate) fn is_local(&self) -> bool {
        self.local.is_some()
    }
    /// Borrow a saved expression from a resident native table.
    /// This performs no file access, symbolic work or evaluator compilation.
    /// A parent descriptor can name a table that this selected owner has not
    /// loaded; requesting it returns an error rather than importing more data.
    pub fn native_atom(
        &self,
        table: &m::Digest,
        id: m::NativeAtomId,
    ) -> Result<&Atom, KernelError> {
        let values = if let Some(local) = &self.local {
            local
                .values
                .get(table)
                .map(AsRef::as_ref)
                .ok_or_else(|| invalid("nonresident native expression table"))?
        } else {
            if table != &self.manifest.preparation.expressions {
                return Err(invalid("unknown flat expression table"));
            }
            &self.atoms
        };
        values
            .get(id.0)
            .ok_or_else(|| invalid("native expression index"))
    }
    /// Convert the old flat construction to independently resident native tables.
    /// Only index projection occurs here; no algebra or expression expansion.
    pub(crate) fn into_local(mut self) -> Result<Self, KernelError> {
        if self.local.is_some() {
            return Ok(self);
        }
        let mut manifest = self.manifest.as_ref().clone();
        let mut tables = BTreeMap::new();
        let mut values = Vec::new();
        let mut remap = BTreeMap::new();
        for binding in &mut manifest.preparation.fiber.bindings {
            atom_id(&self.atoms, &mut binding.value, &mut remap, &mut values)?;
        }
        manifest.preparation.expressions = insert_table(values, &mut tables)?;
        for patch in &mut manifest.patches {
            localize_map(&mut patch.map, &self.atoms, &mut tables)?;
        }
        for chart in &mut manifest.endpoint_charts {
            localize_map(&mut chart.map, &self.atoms, &mut tables)?;
        }
        let directory = Arc::new(
            tables
                .iter()
                .map(|(id, values)| (id.clone(), values.len()))
                .collect(),
        );
        let descriptor = manifest.descriptor_digest().map_err(invalid)?;
        for record in &mut self.records {
            record.manifest = descriptor.clone();
        }
        self.resident.manifest = descriptor;
        self.manifest = Arc::new(manifest);
        self.atoms.clear();
        self.atoms.shrink_to_fit();
        self.local = Some(LocalTables {
            directory,
            values: tables,
        });
        self.validate_local_tables()?;
        Ok(self)
    }
    fn required_tables(&self) -> Result<BTreeSet<m::Digest>, KernelError> {
        let mut required = BTreeSet::from([self.manifest.preparation.expressions.clone()]);
        let ids = self
            .records
            .iter()
            .filter(|r| matches!(r.kind, m::RecordKind::Stochastic { .. }))
            .flat_map(|r| &r.contributions)
            .copied()
            .collect::<BTreeSet<_>>();
        // Zero witnesses remain in the closed directory; no evaluator or native
        // map values are needed for a zero-in-requested-layout contribution.
        for contribution in self
            .manifest
            .contributions
            .iter()
            .filter(|c| ids.contains(&c.id))
        {
            let chart = self
                .manifest
                .endpoint_charts
                .iter()
                .find(|c| c.id == contribution.chart)
                .ok_or_else(|| invalid("missing contribution chart"))?;
            required.insert(chart.map.geometry().expressions.clone());
            let patch = match chart.origin {
                m::EndpointOrigin::ExactPatch(id) => id,
                m::EndpointOrigin::Cell(id) => {
                    self.manifest
                        .cells
                        .iter()
                        .find(|c| c.id == id)
                        .ok_or_else(|| invalid("missing contribution cell"))?
                        .patch
                }
            };
            let patch = self
                .manifest
                .patches
                .iter()
                .find(|p| p.id == patch)
                .ok_or_else(|| invalid("missing contribution patch"))?;
            required.insert(patch.map.geometry().expressions.clone());
        }
        Ok(required)
    }
    pub(super) fn validate_local_tables(&self) -> Result<(), KernelError> {
        let Some(local) = &self.local else {
            return Ok(());
        };
        for (id, values) in &local.values {
            if local.directory.get(id) != Some(&values.len()) {
                return Err(invalid("native table directory/values mismatch"));
            }
            if &table_digest(values) != id {
                return Err(invalid("native local table digest mismatch"));
            }
        }
        if self
            .required_tables()?
            .iter()
            .any(|id| !local.values.contains_key(id))
        {
            return Err(invalid("missing resident native map table"));
        }
        Ok(())
    }
    pub(crate) fn selected(&self, selection: m::ResidentSelection) -> Result<Self, KernelError> {
        if self.local.is_none() {
            return self.clone().into_local()?.selected(selection);
        }
        let mut selected = self.clone();
        match &selection {
            m::ResidentSelection::Complete => {}
            m::ResidentSelection::Selected {
                stochastic_contributions,
                exact_policy,
            } => {
                use crate::results::ExactContributionPolicy;
                selected.records.retain(|r| match &r.kind {
                    m::RecordKind::Exact => *exact_policy == ExactContributionPolicy::IncludeAll,
                    m::RecordKind::Stochastic { .. } => r
                        .contributions
                        .iter()
                        .all(|c| stochastic_contributions.contains(c)),
                });
            }
        }
        selected.resident.selection = selection;
        selected
            .manifest
            .validate_structure(&selected.tables(), m::Limits::default())
            .map_err(invalid)?
            .validate_resident_records(
                &selected.resident.manifest,
                &selected.resident,
                &selected.records,
            )
            .map_err(invalid)?;
        let needed = selected.required_tables()?;
        selected
            .local
            .as_mut()
            .unwrap()
            .values
            .retain(|id, _| needed.contains(id));
        selected.validate_local_tables()?;
        Ok(selected)
    }
    pub(crate) fn save_v16_with_parent(&self, inline: bool) -> Result<SavedV16, KernelError> {
        let local = self
            .local
            .as_ref()
            .ok_or_else(|| invalid("v16 requires local tables"))?;
        self.validate_local_tables()?;
        Ok(SavedV16 {
            parent: if inline {
                serde_json::to_vec(&Parent {
                    manifest: self.manifest.as_ref().clone(),
                    directory: local.directory.as_ref().clone(),
                    semantic: self.semantic.to_string(),
                    orders: self.orders.as_ref().clone(),
                })?
            } else {
                Vec::new()
            },
            parent_id: self.resident.manifest.0.clone(),
            residency: serde_json::to_vec(&Residency {
                records: self.records.clone(),
                resident: self.resident.clone(),
            })?,
            values: local
                .values
                .iter()
                .map(|(id, values)| (id.0.clone(), values.as_ref().clone()))
                .collect(),
            symbols: self.symbols.clone(),
        })
    }
    pub(crate) fn restore_v16_with_parent(
        saved: SavedV16,
        supplied: Option<&Self>,
    ) -> Result<Self, KernelError> {
        let parent = if saved.parent.is_empty() {
            let supplied = supplied.ok_or_else(|| {
                invalid("v16 reference record requires its native archive parent")
            })?;
            if supplied.resident.manifest.0 != saved.parent_id || supplied.symbols != saved.symbols
            {
                return Err(invalid("native archive parent association"));
            }
            None
        } else {
            Some(serde_json::from_slice::<Parent>(&saved.parent)?)
        };
        let residency: Residency = serde_json::from_slice(&saved.residency)?;
        let mut values = BTreeMap::new();
        for (id, atoms) in saved.values {
            if values.insert(m::Digest(id), Arc::new(atoms)).is_some() {
                return Err(invalid("duplicate native local table"));
            }
        }
        let (manifest, directory, semantic, orders) = if let Some(parent) = parent {
            (
                Arc::new(parent.manifest),
                Arc::new(parent.directory),
                Arc::from(parent.semantic),
                Arc::new(parent.orders),
            )
        } else {
            let parent = supplied.unwrap();
            (
                parent.manifest.clone(),
                parent
                    .local
                    .as_ref()
                    .ok_or_else(|| invalid("archive parent must use local tables"))?
                    .directory
                    .clone(),
                parent.semantic.clone(),
                parent.orders.clone(),
            )
        };
        if manifest.descriptor_digest().map_err(invalid)?.0 != saved.parent_id {
            return Err(invalid("v16 parent descriptor reference"));
        }
        let value = Self {
            parameter_guard: None,
            manifest,
            atoms: Vec::new(),
            local: Some(LocalTables { directory, values }),
            symbols: saved.symbols,
            records: residency.records,
            resident: residency.resident,
            semantic,
            orders,
        };
        value.validate_local_tables()?;
        Ok(value)
    }
}

#[cfg(feature = "threshold-decomposition")]
mod staging {
    use super::*;
    use crate::generation::streaming::{
        RecordRef,
        codec::{self, Atoms},
    };
    use std::path::Path;
    const KIND: &str = "threshold-local-native-table-v1";
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Table {
        identity: m::Digest,
        count: usize,
    }
    /// Private, live preparation-issued record plan. No global native Atom,
    /// FunctionMap, continuation or GCAD owner is retained.
    #[derive(Clone)]
    pub(crate) struct StagedTables {
        owner: ThresholdMetadata,
        refs: BTreeMap<m::Digest, RecordRef>,
        guard: Option<crate::kernel::parameter_guard::staging::StagedGuard>,
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct StagedTablePlan {
        parent: Parent,
        records: Vec<m::RecordLineageV1>,
        refs: Vec<(m::Digest, RecordRef)>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        guard: Option<crate::kernel::parameter_guard::staging::StagedGuard>,
    }
    impl ThresholdMetadata {
        pub(crate) fn stage_tables(mut self, root: &Path) -> Result<StagedTables, KernelError> {
            self = self.into_local()?;
            let mut refs = BTreeMap::new();
            let values = std::mem::take(&mut self.local.as_mut().unwrap().values);
            for (identity, atoms) in values {
                let record = codec::write(
                    root,
                    &format!("native-table-{}", identity.0),
                    KIND,
                    &Table {
                        identity: identity.clone(),
                        count: atoms.len(),
                    },
                    Atoms {
                        atoms: Arc::unwrap_or_clone(atoms),
                    },
                    self.symbols.clone(),
                )
                .map_err(invalid)?;
                refs.insert(identity, record);
            }
            let guard = self
                .parameter_guard
                .take()
                .map(|guard| {
                    crate::kernel::parameter_guard::staging::write(root, &guard).map_err(invalid)
                })
                .transpose()?;
            Ok(StagedTables {
                owner: self,
                refs,
                guard,
            })
        }
    }
    impl StagedTables {
        pub(crate) fn descriptor(&self) -> (StagedTablePlan, Vec<Symbol>) {
            let local = self.owner.local.as_ref().expect("staged local owner");
            (
                StagedTablePlan {
                    guard: self.guard.clone(),
                    parent: Parent {
                        manifest: self.owner.manifest.as_ref().clone(),
                        directory: local.directory.as_ref().clone(),
                        semantic: self.owner.semantic.to_string(),
                        orders: self.owner.orders.as_ref().clone(),
                    },
                    records: self.owner.records.clone(),
                    refs: self
                        .refs
                        .iter()
                        .map(|(id, record)| (id.clone(), record.clone()))
                        .collect(),
                },
                self.owner.symbols.clone(),
            )
        }
        /// Untrusted worker transport; only the issuing live plan can authorize
        /// completed records for a full publication. No native proof is decoded.
        pub(crate) fn from_descriptor(
            plan: StagedTablePlan,
            symbols: Vec<Symbol>,
        ) -> Result<Self, KernelError> {
            let parent = plan.parent;
            let descriptor = parent.manifest.descriptor_digest().map_err(invalid)?;
            let owner = ThresholdMetadata {
                parameter_guard: None,
                manifest: Arc::new(parent.manifest),
                atoms: Vec::new(),
                local: Some(LocalTables {
                    directory: Arc::new(parent.directory),
                    values: BTreeMap::new(),
                }),
                symbols,
                records: plan.records,
                resident: m::ResidentLineageV1 {
                    manifest: descriptor,
                    selection: m::ResidentSelection::Complete,
                },
                semantic: Arc::from(parent.semantic),
                orders: Arc::new(parent.orders),
            };
            owner
                .manifest
                .validate_structure(&owner.tables(), m::Limits::default())
                .map_err(invalid)?
                .validate_resident_records(
                    &owner.resident.manifest,
                    &owner.resident,
                    &owner.records,
                )
                .map_err(invalid)?;
            let mut refs = BTreeMap::new();
            for (id, record) in plan.refs {
                if refs.insert(id, record).is_some() {
                    return Err(invalid("duplicate staged native table reference"));
                }
            }
            if !refs
                .keys()
                .eq(owner.local.as_ref().unwrap().directory.keys())
            {
                return Err(invalid("incomplete staged table directory"));
            }
            if owner
                .manifest
                .preparation
                .runtime_chamber
                .as_ref()
                .map(|c| c.guard_semantic.0.as_str())
                != plan.guard.as_ref().map(|g| g.semantic.as_str())
            {
                return Err(invalid("staged parameter guard association"));
            }
            Ok(Self {
                owner,
                refs,
                guard: plan.guard,
            })
        }
        pub(crate) fn publication(
            &self,
            settings: super::super::super::CompilationSettings,
            precision: &super::super::super::PrecisionPolicy,
        ) -> Result<crate::kernel::indexed::ThresholdArchiveSummary, KernelError> {
            use crate::kernel::indexed::{ThresholdArchiveSummary, ThresholdExpectedRecord};
            let policy = crate::kernel::artifact::compiler_policy_with_settings(settings);
            let mut expected = Vec::new();
            let mut selections = vec![(
                true,
                m::ResidentSelection::Selected {
                    stochastic_contributions: Vec::new(),
                    exact_policy: crate::results::ExactContributionPolicy::IncludeAll,
                },
            )];
            selections.extend(self.owner.manifest.contributions.iter().filter_map(
                |c| match c.kind {
                    m::ContributionKind::Stochastic { .. } => Some((
                        false,
                        m::ResidentSelection::Selected {
                            stochastic_contributions: vec![c.id],
                            exact_policy: crate::results::ExactContributionPolicy::ExcludeAll,
                        },
                    )),
                    _ => None,
                },
            ));
            for (carrier, selection) in selections {
                let mut owner = self.owner.clone();
                if let m::ResidentSelection::Selected {
                    stochastic_contributions,
                    exact_policy,
                } = &selection
                {
                    owner.records.retain(|r| match r.kind {
                        m::RecordKind::Exact => {
                            *exact_policy == crate::results::ExactContributionPolicy::IncludeAll
                        }
                        m::RecordKind::Stochastic { .. } => r
                            .contributions
                            .iter()
                            .all(|id| stochastic_contributions.contains(id)),
                    });
                }
                owner.resident.selection = selection;
                expected.push(ThresholdExpectedRecord {
                    carrier,
                    lineage: owner.combined_record_lineage()?,
                    native_content_id: owner.semantic_identity(&policy, precision)?,
                });
            }
            Ok(ThresholdArchiveSummary {
                runtime_guard: crate::kernel::indexed::ThresholdRuntimeGuard::from_owner(
                    &self.owner,
                )?,
                parent: self.owner.resident.manifest.clone(),
                source_identity: self.owner.manifest.preparation.source_identity.0.clone(),
                source_extent: self.owner.manifest.preparation.extent.clone(),
                contributions: self.owner.manifest.contributions.clone(),
                complete_content_id: self.owner.semantic_identity(&policy, precision)?,
                expected,
            })
        }
        pub(crate) fn source_identity(&self) -> &str {
            &self.owner.manifest.preparation.source_identity.0
        }
        pub(crate) fn manifest(&self) -> &m::LineageManifestV1 {
            &self.owner.manifest
        }
        pub(crate) fn runtime_parameters(&self) -> Result<Vec<Symbol>, KernelError> {
            self.owner.runtime_parameters()
        }
        pub(crate) fn orders(&self) -> &[i32] {
            &self.owner.orders
        }
        pub(crate) fn load(
            &self,
            root: &Path,
            selection: m::ResidentSelection,
            limit: u64,
        ) -> Result<ThresholdMetadata, KernelError> {
            // Select records structurally first. No missing local values are
            // treated as a proof failure until this request's tables are read.
            let mut owner = self.owner.clone();
            if let m::ResidentSelection::Selected {
                stochastic_contributions,
                exact_policy,
            } = &selection
            {
                owner.records.retain(|r| match &r.kind {
                    m::RecordKind::Exact => {
                        *exact_policy == crate::results::ExactContributionPolicy::IncludeAll
                    }
                    m::RecordKind::Stochastic { .. } => r
                        .contributions
                        .iter()
                        .all(|c| stochastic_contributions.contains(c)),
                });
            }
            owner.resident.selection = selection;
            owner
                .manifest
                .validate_structure(&owner.tables(), m::Limits::default())
                .map_err(invalid)?
                .validate_resident_records(
                    &owner.resident.manifest,
                    &owner.resident,
                    &owner.records,
                )
                .map_err(invalid)?;
            let required = owner.required_tables()?;
            let mut total = 0u64;
            if let Some(guard) = &self.guard {
                total = total
                    .checked_add(guard.record.bytes)
                    .filter(|n| *n <= limit)
                    .ok_or_else(|| invalid("parameter guard transport limit"))?;
                let expected = &owner
                    .manifest
                    .preparation
                    .runtime_chamber
                    .as_ref()
                    .ok_or_else(|| invalid("guard without uniform chamber"))?
                    .guard_semantic
                    .0;
                let restored =
                    crate::kernel::parameter_guard::staging::read(root, guard, expected, limit)
                        .map_err(invalid)?;
                owner.attach_parameter_guard(restored)?;
            }
            for id in required {
                let record = self
                    .refs
                    .get(&id)
                    .ok_or_else(|| invalid("missing staged native table"))?;
                total = total
                    .checked_add(record.bytes)
                    .filter(|n| *n <= limit)
                    .ok_or_else(|| invalid("native table transport limit"))?;
                let (table, atoms, symbols): (Table, _, _) =
                    codec::read(root, record, KIND).map_err(invalid)?;
                if table.identity != id
                    || table.count != atoms.atoms.len()
                    || symbols != owner.symbols
                {
                    return Err(invalid("native table record association"));
                }
                owner
                    .local
                    .as_mut()
                    .unwrap()
                    .values
                    .insert(id, Arc::new(atoms.atoms));
            }
            owner.validate_local_tables()?;
            Ok(owner)
        }
    }
}
#[cfg(feature = "threshold-decomposition")]
pub(crate) use staging::{StagedTablePlan, StagedTables};

impl ThresholdMetadata {
    pub(crate) fn merge_local(&mut self, other: &Self) -> Result<(), KernelError> {
        self.validate_parameter_guard(&self.runtime_parameters()?)?;
        other.validate_parameter_guard(&other.runtime_parameters()?)?;
        let a = self
            .local
            .as_mut()
            .ok_or_else(|| invalid("merge requires local threshold tables"))?;
        let b = other
            .local
            .as_ref()
            .ok_or_else(|| invalid("merge requires local threshold tables"))?;
        if self.manifest != other.manifest
            || self.semantic != other.semantic
            || self.orders != other.orders
            || self.symbols != other.symbols
            || a.directory != b.directory
        {
            return Err(invalid("foreign threshold native parent"));
        }
        let mut seen = self
            .records
            .iter()
            .flat_map(|r| r.contributions.iter().copied())
            .collect::<BTreeSet<_>>();
        for record in &other.records {
            if record.contributions.iter().any(|id| !seen.insert(*id)) {
                return Err(invalid("duplicate native threshold contribution"));
            }
            self.records.push(record.clone());
        }
        self.records
            .sort_by(|a, b| a.contributions.cmp(&b.contributions));
        for (id, values) in &b.values {
            if a.values.get(id).is_some_and(|old| old != values) {
                return Err(invalid("different native values for one table"));
            }
            a.values.entry(id.clone()).or_insert_with(|| values.clone());
        }
        let includes = |selection: &m::ResidentSelection| {
            matches!(
                selection,
                m::ResidentSelection::Complete
                    | m::ResidentSelection::Selected {
                        exact_policy: crate::results::ExactContributionPolicy::IncludeAll,
                        ..
                    }
            )
        };
        self.resident.selection = m::ResidentSelection::Selected {
            stochastic_contributions: self
                .records
                .iter()
                .filter(|r| matches!(r.kind, m::RecordKind::Stochastic { .. }))
                .flat_map(|r| r.contributions.iter().copied())
                .collect(),
            exact_policy: if includes(&self.resident.selection)
                || includes(&other.resident.selection)
            {
                crate::results::ExactContributionPolicy::IncludeAll
            } else {
                crate::results::ExactContributionPolicy::ExcludeAll
            },
        };
        self.validate_local_tables()?;
        Ok(())
    }
}

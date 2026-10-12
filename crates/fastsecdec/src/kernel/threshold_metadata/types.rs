use crate::generation::SourceSectorSelection;
use serde::{Deserialize, Serialize};

macro_rules! index {
    ($($name:ident),+ $(,)?) => {$(
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub usize);
    )+};
}
index!(
    NativeAtomId,
    NativeSymbolId,
    PatchId,
    CellId,
    EndpointChartId,
    GroupId,
    ContributionId
);

/// Content digest of an associated native record or canonical semantic witness.
/// The field's parent specifies which identity domain it belongs to.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Digest(pub String);

/// Compact schema bounds for independently stored native expression records.
/// No Atom, evaluator or global GCAD proof is loaded to use these bounds. Symbol
/// IDs refer to the common lightweight preparation symbol-role inventory;
/// per-record native State imports must check that association separately.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeTables {
    pub expression_records: std::collections::BTreeMap<Digest, usize>,
    pub symbols: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceExtent {
    Full {
        ordinary_geometry: Option<OrdinaryGeometry>,
    },
    Selected {
        selection: SourceSectorSelection,
        ordinary_geometry: Digest,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrdinaryGeometry {
    pub identity: Digest,
    pub original_source_count: usize,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedStrategy {
    GcadFirst,
    SectorFirst,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RationalBinding {
    pub symbol: NativeSymbolId,
    /// Must decode to a native exact real Rational; index validity alone does
    /// not establish that condition.
    pub value: NativeAtomId,
}

/// Fixed physical bindings. This descriptor alone licenses no runtime rebind;
/// a separate explicit uniform chamber is required for remaining runtime roles.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixedRationalFiber {
    pub physical_parameters: Vec<NativeSymbolId>,
    pub bindings: Vec<RationalBinding>,
    pub admission: Digest,
}

/// Role of one exact predicate in the issued parameter-prefix checker.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterPredicateRole {
    LowerBound,
    UpperBound,
    StrictPositive,
    Nonzero,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParameterPredicateV1 {
    pub role: ParameterPredicateRole,
    /// Raw-prefix axis for a bound, otherwise position within this role's list.
    pub index: usize,
    pub positive_selector: bool,
    /// Canonical exact N/D pair or polynomial with its public parameter roles.
    pub predicate: Digest,
}
/// Compact association to a source-issued complete uniform family. Reading
/// these fields restores no geometry proof. Native executable records must
/// separately own the exact predicate program with this semantic identity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UniformParameterChamberV1 {
    pub version: u32,
    pub runtime_parameters: Vec<NativeSymbolId>,
    pub prefix_to_public: Vec<usize>,
    /// Raw CAD axes in original integration-coordinate order; never sample the
    /// parameter prefix or reinterpret it as an integration dimension.
    pub integration_axes: Vec<usize>,
    pub predicates: Vec<ParameterPredicateV1>,
    pub guard_semantic: Digest,
    pub family_certificate: Digest,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preparation {
    pub source_identity: Digest,
    pub extent: SourceExtent,
    pub strategy: ResolvedStrategy,
    pub actual_options: Digest,
    pub expressions: Digest,
    /// Global original-extent to patch atlas association. Local patch/cell
    /// certificates alone do not assert that this atlas covers the input once.
    pub partition_certificate: Digest,
    pub original_coordinates: Vec<NativeSymbolId>,
    pub regulator: NativeSymbolId,
    pub fiber: FixedRationalFiber,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_chamber: Option<UniformParameterChamberV1>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "index",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SourceOrigin {
    OriginalInput,
    OrdinarySector(usize),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    Forward,
    Reversed,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootDomain {
    RealAscending,
    RealDescending,
    Positive,
}

/// Exact selector data, never a sample-fiber isolation enclosure. Generation
/// admits the native polynomial and selector against its geometric owner before
/// constructing callbacks. Saved descriptors restore arithmetic association,
/// without issuing a new geometric certificate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootSection {
    pub polynomial: NativeAtomId,
    pub root_variable: NativeSymbolId,
    pub preceding_coordinates: Vec<NativeSymbolId>,
    pub domain: RootDomain,
    pub ordinal: usize,
    pub branch_certificate: Digest,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapGeometry {
    /// Independently loadable native Atom record; indices below are local to it.
    pub expressions: Digest,
    pub coordinates: Vec<NativeSymbolId>,
    pub images: Vec<NativeAtomId>,
    pub positive_measure: NativeAtomId,
    pub orientation: Orientation,
    pub certificate: Digest,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MapDescriptor {
    RationalV1 {
        geometry: MapGeometry,
    },
    /// Inspectable future descriptor. Initial executable admission refuses it.
    AlgebraicSectionsV1 {
        geometry: MapGeometry,
        sections: Vec<RootSection>,
    },
    /// A saved fixed regular-section continuation issued by the sealed native
    /// factory. Loading this arithmetic record does not replay global proofs.
    RegularSecantV1 {
        geometry: MapGeometry,
        sections: Vec<RootSection>,
        endpoint: NormalizedEndpointV1,
    },
    /// Complete cell maps followed by certified endpoint charts. Unlike a
    /// last-axis secant, every image may depend on all chart coordinates.
    /// Native issuers retain the coverage and closed-face proof authority;
    /// this saved descriptor carries their arithmetic and inspection lineage.
    ResolvedEndpointV1 {
        geometry: MapGeometry,
        sections: Vec<RootSection>,
        endpoint: NormalizedEndpointV1,
    },
}
impl MapDescriptor {
    pub fn geometry(&self) -> &MapGeometry {
        match self {
            Self::RationalV1 { geometry }
            | Self::AlgebraicSectionsV1 { geometry, .. }
            | Self::RegularSecantV1 { geometry, .. }
            | Self::ResolvedEndpointV1 { geometry, .. } => geometry,
        }
    }

    pub(crate) fn sections(&self) -> Option<&[RootSection]> {
        match self {
            Self::RationalV1 { .. } => None,
            Self::AlgebraicSectionsV1 { sections, .. }
            | Self::RegularSecantV1 { sections, .. }
            | Self::ResolvedEndpointV1 { sections, .. } => Some(sections),
        }
    }

    pub(crate) fn endpoint(&self) -> Option<&NormalizedEndpointV1> {
        match self {
            Self::RegularSecantV1 { endpoint, .. } | Self::ResolvedEndpointV1 { endpoint, .. } => {
                Some(endpoint)
            }
            Self::RationalV1 { .. } | Self::AlgebraicSectionsV1 { .. } => None,
        }
    }
}

/// All Atom references belong to this chart's independently loadable map table.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NormalizedEndpointV1 {
    pub regulator: NativeSymbolId,
    pub terms: Vec<NormalizedTermV1>,
    pub continuation_policy: Digest,
    pub common_domain: Digest,
    pub numerator_functions: Digest,
    pub root_helpers: Vec<Digest>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NormalizedTermV1 {
    pub source_term: usize,
    pub prefactor: NativeAtomId,
    pub regular: NativeAtomId,
    pub powers: Vec<NativeAtomId>,
    pub factors: Vec<NormalizedFactorV1>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NormalizedFactorV1 {
    pub source_term: usize,
    pub source_factor: usize,
    pub source: NativeAtomId,
    pub exponent: NativeAtomId,
    pub certificate: Digest,
}
impl NormalizedEndpointV1 {
    pub(crate) fn map_atoms<E>(
        &mut self,
        mut f: impl FnMut(&mut NativeAtomId) -> Result<(), E>,
    ) -> Result<(), E> {
        for term in &mut self.terms {
            f(&mut term.prefactor)?;
            f(&mut term.regular)?;
            for power in &mut term.powers {
                f(power)?;
            }
            for factor in &mut term.factors {
                f(&mut factor.source)?;
                f(&mut factor.exponent)?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Patch {
    pub id: PatchId,
    pub source: SourceOrigin,
    pub map: MapDescriptor,
    pub coverage: Digest,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeCellLocator {
    pub request_record: Digest,
    pub raw_evidence: Digest,
    pub raw_cell_index: usize,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cell {
    pub id: CellId,
    pub patch: PatchId,
    pub native: NativeCellLocator,
    pub coordinate_order: Vec<NativeSymbolId>,
    pub coverage: Digest,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EndpointChart {
    pub id: EndpointChartId,
    pub origin: EndpointOrigin,
    pub path: Vec<usize>,
    pub map: MapDescriptor,
    pub endpoint_certificate: Digest,
    /// Binds original signed causal factors, complete exponents and their
    /// boundary-value phase, including the direct zero-dimensional path.
    pub causal_phase_certificate: Digest,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "id",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum EndpointOrigin {
    Cell(CellId),
    /// A genuine zero-dimensional preparation has no GCAD problem or cell.
    ExactPatch(PatchId),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Continuation {
    EpsilonStripV1 {
        certificate: Digest,
    },
    /// Planned general capability, not admitted by the first factory.
    AuxiliaryIntegralCancellationV1 {
        certificate: Digest,
    },
}
impl Continuation {
    pub fn certificate(&self) -> &Digest {
        match self {
            Self::EpsilonStripV1 { certificate }
            | Self::AuxiliaryIntegralCancellationV1 { certificate } => certificate,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContinuationGroup {
    pub id: GroupId,
    pub charts: Vec<EndpointChartId>,
    pub continuation: Continuation,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ContributionKind {
    Stochastic {
        coordinates: Vec<NativeSymbolId>,
    },
    Exact,
    /// Requires continuation/coverage evidence, not CAD codimension alone.
    CertifiedZero {
        certificate: Digest,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contribution {
    pub id: ContributionId,
    pub chart: EndpointChartId,
    pub group: GroupId,
    pub kind: ContributionKind,
}

/// An untrusted data-only directory. validate_structure checks associations and
/// coverage inventories; it does not reproduce GCAD/endpoint/continuation proof.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LineageManifestV1 {
    pub version: u32,
    pub preparation: Preparation,
    pub patches: Vec<Patch>,
    pub cells: Vec<Cell>,
    pub endpoint_charts: Vec<EndpointChart>,
    pub continuation_groups: Vec<ContinuationGroup>,
    pub contributions: Vec<Contribution>,
}

/// One record can own several exact contributions or one stochastic program's
/// contributions. Zero leaves live only in the global manifest.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RecordKind {
    Exact,
    Stochastic { coordinates: Vec<NativeSymbolId> },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordLineageV1 {
    pub manifest: Digest,
    pub contributions: Vec<ContributionId>,
    pub kind: RecordKind,
}

/// Native residency is distinct from the physical source extent. An explicit
/// selection remains partial even if it lists every stochastic contribution.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResidentSelection {
    Complete,
    Selected {
        stochastic_contributions: Vec<ContributionId>,
        exact_policy: crate::results::ExactContributionPolicy,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResidentLineageV1 {
    pub manifest: Digest,
    pub selection: ResidentSelection,
}

/// Scope of validation requested by a caller, independent of numeric stability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationMode {
    Structural,
    GlobalProof,
}

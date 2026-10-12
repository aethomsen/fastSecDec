//! Typed dispatch retains each genuine request and verified source owner.
use super::*;
use gcad::staging::{
    EvidenceRecord, ObservationRecord, RequestRecord, StagedPositiveRequest, StagedRequest,
};

pub(super) enum Request {
    Affine(Arc<gcad::GcadRequest>),
    Positive(Arc<gcad::PositiveProjectiveRequest>),
}
pub(super) enum Staged {
    Affine(StagedRequest),
    Positive(StagedPositiveRequest),
}
pub(super) enum Raw {
    Affine(Box<gcad::NativeDecomposition>),
    Positive(Box<symgcad::output::SolveResult>),
}
pub(super) enum VerifiedSource {
    Affine(Arc<gcad::VerifiedDecomposition>),
    Positive(Arc<gcad::PositiveProjectiveQuotient>),
}
pub(super) struct VerificationAttempt {
    pub observation: ObservationRecord,
    pub result: std::result::Result<VerifiedSource, gcad::GcadError>,
}
impl Request {
    pub fn dimensions(&self) -> usize {
        match self {
            Self::Affine(r) => r.domain().coordinates().len(),
            Self::Positive(r) => r.preparation().coordinates().len(),
        }
    }
    pub fn occupied(&self) -> std::collections::BTreeSet<Symbol> {
        match self {
            Self::Affine(r) => super::super::maps::coordinate_symbols(r),
            Self::Positive(r) => {
                use symbolica::atom::AtomCore;
                let input = r.preparation().input();
                let mut names = input
                    .parameters()
                    .iter()
                    .copied()
                    .collect::<std::collections::BTreeSet<_>>();
                names.insert(input.regulator());
                names.insert(symbolica::transcendental::root_var());
                for term in input.terms() {
                    names.extend(term.prefactor().get_all_symbols(true));
                    for power in term.monomial_powers() {
                        names.extend(power.get_all_symbols(true));
                    }
                    for factor in term.factors() {
                        names.extend(factor.polynomial().get_all_symbols(true));
                        names.extend(factor.exponent().get_all_symbols(true));
                    }
                }
                names
            }
        }
    }
}
impl Staged {
    pub fn write(root: &Path, request: Request) -> Result<Self> {
        Ok(match request {
            Request::Affine(r) => Self::Affine(StagedRequest::write(root, r)?),
            Request::Positive(r) => Self::Positive(StagedPositiveRequest::write(root, r)?),
        })
    }
    pub fn request(&self) -> Request {
        match self {
            Self::Affine(s) => Request::Affine(s.request_owner().clone()),
            Self::Positive(s) => Request::Positive(s.request_owner().clone()),
        }
    }
    pub fn receipt(&self) -> &RequestRecord {
        match self {
            Self::Affine(s) => s.receipt(),
            Self::Positive(s) => s.receipt(),
        }
    }
    pub fn solve(&self) -> gcad::Result<Raw> {
        match self {
            Self::Affine(s) => s.request().solve().map(|raw| Raw::Affine(Box::new(raw))),
            Self::Positive(s) => s.request().solve().map(|raw| Raw::Positive(Box::new(raw))),
        }
    }
    pub fn write_evidence(&self, root: &Path, raw: &Raw) -> Result<EvidenceRecord> {
        Ok(match (self, raw) {
            (Self::Affine(s), Raw::Affine(raw)) => s.write_evidence(root, raw)?,
            (Self::Positive(s), Raw::Positive(raw)) => s.write_evidence(root, raw)?,
            _ => return Err(Error::Association("native evidence/request variant")),
        })
    }
    pub fn verify_evidence(
        &self,
        root: &Path,
        raw: &EvidenceRecord,
        limit: u64,
    ) -> Result<VerificationAttempt> {
        Ok(match self {
            Self::Affine(s) => {
                let v = s.verify_evidence(root, raw, limit)?;
                VerificationAttempt {
                    observation: v.observation,
                    result: v.result.map(|x| VerifiedSource::Affine(Arc::new(x))),
                }
            }
            Self::Positive(s) => {
                let v = s.verify_evidence(root, raw, limit)?;
                VerificationAttempt {
                    observation: v.observation,
                    result: v.result.map(|x| VerifiedSource::Positive(Arc::new(x))),
                }
            }
        })
    }
    pub fn read<F: FnMut(&Progress) -> ControlFlow<()>>(
        root: &Path,
        receipt: &RequestRecord,
        maximum_bytes: u64,
        route: regular::Route,
        original: Option<(OriginalPoint, represented::Limits)>,
        reporter: &mut Reporter<'_, F>,
    ) -> Result<Self> {
        // The retained route selects a codec; both codecs independently verify
        // request kind, source identity and original graph conversion ownership.
        let positive = route == regular::Route::PositiveProjective;
        let result = match original {
            Some((OriginalPoint::Graph(original), cap)) => {
                let progress = |p: represented::graph::Progress| {
                    reporter.graph_input = Some(p);
                    if reporter.poll(p.converted_literals, None).is_ok() {
                        ControlFlow::Continue(())
                    } else {
                        ControlFlow::Break(())
                    }
                };
                if positive {
                    StagedPositiveRequest::read_with_original_graph(
                        root,
                        receipt,
                        maximum_bytes,
                        original,
                        cap,
                        progress,
                    )
                    .map(Self::Positive)
                } else {
                    StagedRequest::read_with_original_graph(
                        root,
                        receipt,
                        maximum_bytes,
                        original,
                        cap,
                        progress,
                    )
                    .map(Self::Affine)
                }
            }
            Some((OriginalPoint::Family(original), cap)) => {
                if positive {
                    return Err(Error::Unsupported(
                        "positive-projective family provenance is not implemented",
                    ));
                }
                StagedRequest::read_with_original_family(
                    root,
                    receipt,
                    maximum_bytes,
                    original,
                    cap,
                    |p| {
                        reporter.family_input = Some(p);
                        if reporter.poll(p.converted_literals, None).is_ok() {
                            ControlFlow::Continue(())
                        } else {
                            ControlFlow::Break(())
                        }
                    },
                )
                .map(Self::Affine)
            }
            None if positive => {
                StagedPositiveRequest::read(root, receipt, maximum_bytes).map(Self::Positive)
            }
            None => StagedRequest::read(root, receipt, maximum_bytes).map(Self::Affine),
        };
        result.map_err(|error| match error {
            gcad::staging::Error::Gcad(gcad::GcadError::Represented(
                represented::Error::Cancelled,
            )) => Error::Cancelled,
            error => Error::Evidence(error),
        })
    }
    pub fn restore_options(&self, options: &mut PreparationOptions) -> Result<()> {
        match self {
            Self::Affine(staged) => {
                if options.graph_payloads.is_some()
                    && staged.request().preparametric_graph_input().is_none()
                {
                    return Err(Error::Association(
                        "graph payload configuration on a non-graph request",
                    ));
                }
                if staged.request().preparametric_graph_input().is_some() {
                    graph::admission(options)?;
                    graph::request_options(options, staged.request())?;
                } else if staged.request().preparametric_family_input().is_some() {
                    family::admission(options)?;
                    family::request_options(options, staged.request())?;
                } else if let Some(owner) = staged.request().represented_input() {
                    let policy = (owner.meaning(), owner.limits());
                    if options.represented.is_some_and(|stored| stored != policy) {
                        return Err(Error::Association("represented policy/configuration"));
                    }
                    options.represented = Some(policy);
                }
            }
            Self::Positive(staged) => match staged.request().provenance() {
                gcad::SourceProvenance::Exact => {
                    if options.represented.is_some() || options.graph_payloads.is_some() {
                        return Err(Error::Association(
                            "positive exact request has represented configuration",
                        ));
                    }
                }
                gcad::SourceProvenance::PreparametricGraph(graph) => {
                    graph::admission(options)?;
                    if options.represented != Some((graph.meaning(), graph.limits()))
                        || options.graph_payloads != graph.payload_policy()
                    {
                        return Err(Error::Association(
                            "positive graph conversion policy/configuration",
                        ));
                    }
                }
                _ => {
                    return Err(Error::Unsupported(
                        "positive represented provenance is not implemented",
                    ));
                }
            },
        }
        Ok(())
    }
}
impl VerifiedSource {
    pub fn dimensions(&self) -> (usize, usize, usize) {
        match self {
            Self::Affine(o) => (
                o.request().input().parameters().len(),
                o.request().domain().coordinates().len(),
                o.cells().len(),
            ),
            Self::Positive(o) => (
                o.preparation().input().parameters().len(),
                o.preparation().coordinates().len(),
                o.cell_count(),
            ),
        }
    }
}

//! Private identity of completed native lower work, retained across partial ascent.
use super::*;
#[derive(Clone, Debug)]
pub(super) enum LowerBranchAnchor {
    Empty(Arc<EtaleFrame>),
    Components(Arc<ContactTerminalComponents>),
    Recursive {
        branch: Arc<ContactDispatchBranch>,
        source: Arc<MarkedIdeal>,
        completion: LowerCompletion,
    },
}
#[derive(Clone, Debug)]
pub(super) enum LowerCompletion {
    Finite(Arc<LocalizedCoverHistory>),
    EmptySupport(Arc<EmptyAmbientSupport>),
    EmptyPrincipal {
        source: Arc<ResolutionHistory>,
        cover: Arc<VerifiedOpenCover>,
        index: usize,
    },
    EmptyChild(Arc<RecursivePresentation>),
}
impl LowerBranchAnchor {
    pub(super) fn capture(work: &ContactBranchWork) -> Option<Self> {
        Some(match work {
            ContactBranchWork::Pending => return None,
            ContactBranchWork::Empty(ContactBranchProduction::EmptyContact {
                evidence: ContactProduction::EmptyContactOpen { graph, .. },
                ..
            }) => Self::Empty(graph.clone()),
            ContactBranchWork::Empty(_) => return None,
            ContactBranchWork::Components(c) => {
                if !c.complete_inventory() {
                    return None;
                }
                Self::Components(c.owner().clone())
            }
            ContactBranchWork::Recursive(c) => {
                if !c.construction_inventory_complete() {
                    return None;
                }
                let completion = match c.ascent()? {
                    RecursiveBranchAscent::Finite(f) => {
                        LowerCompletion::Finite(f.source_open().clone())
                    }
                    RecursiveBranchAscent::EmptySupport(s) => {
                        LowerCompletion::EmptySupport(s.clone())
                    }
                    RecursiveBranchAscent::EmptyPrincipal(r) => match r.as_ref() {
                        CoverLocalizationProduction::Empty {
                            source,
                            cover,
                            index,
                            ..
                        } => LowerCompletion::EmptyPrincipal {
                            source: source.clone(),
                            cover: cover.clone(),
                            index: *index,
                        },
                        _ => return None,
                    },
                    RecursiveBranchAscent::DeferredEmptyChild(
                        RecursiveOutcome::EmptyMarkedCosupport { presentation, .. },
                    ) => LowerCompletion::EmptyChild(presentation.clone()),
                    _ => return None,
                };
                Self::Recursive {
                    branch: c.branch().clone(),
                    source: c.source().clone(),
                    completion,
                }
            }
        })
    }
    pub(super) fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Empty(a), Self::Empty(b)) => Arc::ptr_eq(a, b),
            (Self::Components(a), Self::Components(b)) => Arc::ptr_eq(a, b),
            (
                Self::Recursive {
                    branch: a,
                    source: s,
                    completion: c,
                },
                Self::Recursive {
                    branch: b,
                    source: t,
                    completion: d,
                },
            ) => Arc::ptr_eq(a, b) && Arc::ptr_eq(s, t) && c.same(d),
            _ => false,
        }
    }
}
impl LowerCompletion {
    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Finite(a), Self::Finite(b)) => Arc::ptr_eq(a, b),
            (Self::EmptySupport(a), Self::EmptySupport(b)) => Arc::ptr_eq(a, b),
            (Self::EmptyChild(a), Self::EmptyChild(b)) => Arc::ptr_eq(a, b),
            (
                Self::EmptyPrincipal {
                    source: a,
                    cover: c,
                    index: i,
                },
                Self::EmptyPrincipal {
                    source: b,
                    cover: d,
                    index: j,
                },
            ) => Arc::ptr_eq(a, b) && Arc::ptr_eq(c, d) && i == j,
            _ => false,
        }
    }
}

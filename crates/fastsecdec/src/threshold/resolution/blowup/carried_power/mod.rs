//! Retained auxiliary power identity through already-issued local physical maps.
//! This owner neither selects a center nor supplies a real integration atlas.
use super::super::*;
use super::{
    coefficient::{SupportedMarkedTransform, controlled_supported, transform_supported},
    embedding::SupportEmbedding,
    helpers::{Result, unit},
    lower_boundary::{LowerBoundaryTransform, verify_marked_center},
    support::{StrictContactSupport, StrictSupportOpen, transport_embedding},
};
use std::sync::Arc;
mod first;
#[cfg(test)]
mod tests;
mod transport;

/// Both mates are checked on the same actual frame, even when equivalent.
#[derive(Clone, Debug)]
pub struct PowerCosupportProof {
    frame: Arc<EtaleFrame>,
    original: Arc<MarkedCosupport>,
    effective: Arc<MarkedCosupport>,
}
impl PowerCosupportProof {
    fn prove(
        frame: Arc<EtaleFrame>,
        original: Arc<MarkedIdeal>,
        effective: Arc<MarkedIdeal>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let original = MarkedCosupport::prove(frame.clone(), original, b)?;
        let effective = MarkedCosupport::prove(frame.clone(), effective, b)?;
        if original.empty() != effective.empty() {
            return Err(Error::Invalid(
                "carried power companion cosupport disagreement",
            ));
        }
        Ok(Arc::new(Self {
            frame,
            original,
            effective,
        }))
    }
    pub fn frame(&self) -> &Arc<EtaleFrame> {
        &self.frame
    }
    pub fn original(&self) -> &Arc<MarkedCosupport> {
        &self.original
    }
    pub fn effective(&self) -> &Arc<MarkedCosupport> {
        &self.effective
    }
    pub fn completed(&self) -> bool {
        self.original.empty() && self.effective.empty()
    }
}

/// Completed evidence stays at its last admissible frame; no later pullback.
#[derive(Clone, Debug)]
pub enum CarriedPowerCompanion {
    Active(Arc<PowerCosupportProof>),
    Completed(Arc<PowerCosupportProof>),
}
impl CarriedPowerCompanion {
    fn from_proof(proof: Arc<PowerCosupportProof>) -> Self {
        if proof.completed() {
            Self::Completed(proof)
        } else {
            Self::Active(proof)
        }
    }
    pub fn proof(&self) -> &Arc<PowerCosupportProof> {
        match self {
            Self::Active(p) | Self::Completed(p) => p,
        }
    }
    pub fn completed(&self) -> bool {
        matches!(self, Self::Completed(_))
    }
}

#[derive(Clone, Debug)]
enum PowerStateOrigin {
    First {
        tree: Arc<OriginalTreePullback>,
        level: usize,
        open: usize,
    },
    Continued {
        prior: Arc<CarriedPowerState>,
        chart: Arc<RelativeRecursiveChart>,
    },
}

/// A current auxiliary presentation, not a new initial recursion origin.
#[derive(Clone, Debug)]
pub struct CarriedPowerState {
    origin: PowerStateOrigin,
    initial: Arc<PowerLevelTransform>,
    chart: Arc<RelativeRecursiveChart>,
    support: Arc<StrictContactSupport>,
    open: Arc<StrictSupportOpen>,
    history: Arc<ResolutionHistory>,
    original: Arc<MarkedIdeal>,
    normalized: Arc<MarkedIdeal>,
    effective: Arc<MarkedIdeal>,
    unit: Poly,
    normalizer: Arc<QuotientNormalizer>,
    companion: CarriedPowerCompanion,
    source_cosupport: Arc<PowerCosupportProof>,
    depth: usize,
}
impl CarriedPowerState {
    pub fn initial(&self) -> &Arc<PowerLevelTransform> {
        &self.initial
    }
    pub fn chart(&self) -> &Arc<RelativeRecursiveChart> {
        &self.chart
    }
    pub fn support(&self) -> &Arc<StrictContactSupport> {
        &self.support
    }
    pub fn open(&self) -> &Arc<StrictSupportOpen> {
        &self.open
    }
    pub fn history(&self) -> &Arc<ResolutionHistory> {
        &self.history
    }
    pub fn original(&self) -> &Arc<MarkedIdeal> {
        &self.original
    }
    pub fn normalized(&self) -> &Arc<MarkedIdeal> {
        &self.normalized
    }
    pub fn effective(&self) -> &Arc<MarkedIdeal> {
        &self.effective
    }
    pub fn unit(&self) -> &Poly {
        &self.unit
    }
    pub fn companion(&self) -> &CarriedPowerCompanion {
        &self.companion
    }
    pub fn source_cosupport(&self) -> &Arc<PowerCosupportProof> {
        &self.source_cosupport
    }
    pub fn completed(&self) -> bool {
        self.source_cosupport.completed()
    }
    pub fn normalizer(&self) -> &Arc<QuotientNormalizer> {
        &self.normalizer
    }
    pub fn depth(&self) -> usize {
        self.depth
    }
    pub fn first_slot(&self) -> Option<(&Arc<OriginalTreePullback>, usize, usize)> {
        match &self.origin {
            PowerStateOrigin::First { tree, level, open } => Some((tree, *level, *open)),
            _ => None,
        }
    }
    pub fn prior(&self) -> Option<(&Arc<Self>, &Arc<RelativeRecursiveChart>)> {
        match &self.origin {
            PowerStateOrigin::Continued { prior, chart } => Some((prior, chart)),
            _ => None,
        }
    }
    fn power(&self) -> Result<usize> {
        self.initial
            .presentation()
            .power()
            .map(|p| p.power())
            .ok_or(Error::Invalid("carried power missing original receipt"))
    }
}

/// All strict-support opens of ONE actual physical chart. Empty support retains
/// its saturation receipt; neither this inventory nor its absence is zero density.
#[derive(Clone, Debug)]
pub struct CarriedPowerTransition {
    source: Arc<CarriedPowerState>,
    chart: Arc<RelativeRecursiveChart>,
    support: Arc<StrictContactSupport>,
    states: Vec<Arc<CarriedPowerState>>,
}
impl CarriedPowerTransition {
    pub fn source(&self) -> &Arc<CarriedPowerState> {
        &self.source
    }
    pub fn chart(&self) -> &Arc<RelativeRecursiveChart> {
        &self.chart
    }
    pub fn support(&self) -> &Arc<StrictContactSupport> {
        &self.support
    }
    pub fn states(&self) -> &[Arc<CarriedPowerState>] {
        &self.states
    }
    pub fn complete_inventory(&self) -> bool {
        self.states.len() == self.support.opens().len()
    }
}

fn principal(source: &MarkedIdeal) -> Result<&Poly> {
    match source.ideal().generators() {
        [p] => Ok(p),
        _ => Err(Error::ResourceIncomplete(
            "carried power requires a nonzero principal representative",
        )),
    }
}
fn check_identity(
    frame: &Arc<EtaleFrame>,
    p: &MarkedIdeal,
    g: &MarkedIdeal,
    u: &Poly,
    k: usize,
    b: &mut Budget,
) -> Result<()> {
    if g.mark().checked_mul(k) != Some(p.mark()) {
        return Err(Error::Invalid("carried power integer marks"));
    }
    let local = frame.local();
    if p.ideal().ring() != local.ring() || g.ideal().ring() != local.ring() {
        return Err(Error::Invalid("carried power current ring"));
    }
    local.ring().check(u)?;
    if !unit(local, u, b)? {
        return Err(Error::Invalid("carried power unit"));
    }
    let powered = b.power(principal(g)?, k)?;
    let rhs = b.mul(u, &powered)?;
    if !local.zero(&(principal(p)? - &rhs), b)? {
        return Err(Error::Invalid("carried power exact identity"));
    }
    Ok(())
}

/// Optional auxiliary progress never declares physical density zero. An
/// incomplete step retains the predecessor and exact request for caller retry.
#[derive(Clone, Debug)]
pub enum CarriedPowerAdvance {
    SourceCompleted(Arc<CarriedPowerState>),
    Transformed(Arc<CarriedPowerTransition>),
    Incomplete(Arc<PendingPowerTransition>),
}
#[derive(Clone, Debug)]
pub struct PendingPowerTransition {
    source: Arc<CarriedPowerState>,
    chart: Arc<RelativeRecursiveChart>,
    namespace: String,
    reason: &'static str,
}
impl PendingPowerTransition {
    pub fn source(&self) -> &Arc<CarriedPowerState> {
        &self.source
    }
    pub fn chart(&self) -> &Arc<RelativeRecursiveChart> {
        &self.chart
    }
    pub fn reason(&self) -> &'static str {
        self.reason
    }
    /// Caller-stepped in-memory retry; incomplete native work is recomputed.
    pub fn retry(&self, b: &mut Budget) -> Result<CarriedPowerAdvance> {
        self.source.advance(self.chart.clone(), &self.namespace, b)
    }
}

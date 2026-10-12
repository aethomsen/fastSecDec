//! Bounded proposal discovery. Only the existing complete issuer grants authority.
mod candidates;
use super::*;
use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};
use symbolica::atom::AtomCore;

#[derive(Clone, Debug)]
pub struct DiscoveryOptions {
    pub max_subdivision_depth: u32,
    pub max_attempts: usize,
    pub wall_time: Duration,
}
impl Default for DiscoveryOptions {
    fn default() -> Self {
        Self {
            max_subdivision_depth: 4,
            max_attempts: 64,
            wall_time: Duration::from_secs(300),
        }
    }
}
#[derive(Clone, Debug)]
pub struct CandidateMiss {
    pub cell: usize,
    pub candidate: usize,
    pub error: Error,
}
/// Successful discovery carries the genuine already-issued family, avoiding a
/// duplicate certificate search. The proposals and failed candidates are evidence.
pub struct DiscoveredFamily {
    pub family: ResolvedProjectivePairFamily,
    pub proposals: Vec<CellProposal>,
    pub misses: Vec<CandidateMiss>,
}
fn fresh(occupied: &mut BTreeSet<Symbol>, name: &str) -> Result<Symbol> {
    for i in 0..10_000 {
        let s = Symbol::parse(
            format!("fastsecdec::pair_discovery::{name}_{i}"),
            "fastsecdec",
        )
        .map_err(native)?;
        if s.is_exportable() && occupied.insert(s) {
            return Ok(s);
        }
    }
    Err(Error::ResourceIncomplete(
        "discovery fresh role allowance".into(),
    ))
}
/// Initial automatic class: two-dimensional, constant-first positive CAD,
/// rational linear fibres and simple quadratic pairs with two finite collisions.
/// No source coefficient or topology name chooses a candidate.
pub fn discover(
    owner: Arc<PositiveProjectiveQuotient>,
    coordinates: Vec<Symbol>,
    limits: Limits,
    options: DiscoveryOptions,
    settings: OptimizationSettings,
    mut observer: impl FnMut(Progress) -> ControlFlow<()>,
) -> Result<DiscoveredFamily> {
    if coordinates.len() != 2
        || owner.preparation().coordinates().len() != 2
        || owner.cell_count() == 0
    {
        return Err(unsupported(
            "automatic pair discovery requires two-dimensional nonempty source",
        ));
    }
    if options.max_subdivision_depth > 12
        || options.max_attempts == 0
        || options.wall_time.is_zero()
    {
        return Err(invalid("bounded nonempty discovery options"));
    }
    let started = Instant::now();
    let mut occupied = owner.occupied_symbols().clone();
    occupied.insert(symbolica::transcendental::root_var());
    for s in &coordinates {
        if !s.is_exportable() || !occupied.insert(*s) {
            return Err(invalid("fresh distinct discovery coordinates"));
        }
    }
    let root = fresh(&mut occupied, "root")?;
    let primitive = fresh(&mut occupied, "primitive")?;
    let mut w = Work {
        limits: &limits,
        budget: crate::threshold::maps::regular::Budget::new(limits.algebra.clone()),
        observer: &mut observer,
    };
    let mut portfolios = Vec::new();
    for cell in 0..owner.cell_count() {
        if started.elapsed() >= options.wall_time {
            return Err(Error::ResourceIncomplete("discovery wall allowance".into()));
        }
        w.emit(Progress::Cell(cell))?;
        portfolios.push(candidates::enumerate(
            &owner,
            cell,
            &coordinates,
            root,
            primitive,
            &options,
            &mut w,
        )?);
    }
    drop(w);
    let mut indices = vec![0usize; portfolios.len()];
    let mut misses = Vec::new();
    for _ in 0..options.max_attempts {
        let remaining = options
            .wall_time
            .checked_sub(started.elapsed())
            .ok_or_else(|| Error::ResourceIncomplete("discovery wall allowance".into()))?;
        let proposals = portfolios
            .iter()
            .zip(&indices)
            .map(|(p, i)| p[*i].clone())
            .collect::<Vec<_>>();
        let mut attempt_limits = limits.clone();
        attempt_limits.algebra.wall_time = attempt_limits.algebra.wall_time.min(remaining);
        let mut last_cell = None;
        let mut cancelled = false;
        let result = ResolvedProjectivePairFamily::admit(
            owner.clone(),
            coordinates.clone(),
            proposals.clone(),
            attempt_limits,
            settings.clone(),
            |p| {
                if let Progress::Cell(c) = p {
                    last_cell = Some(c)
                }
                let action = observer(p);
                if action.is_break() {
                    cancelled = true
                }
                action
            },
        );
        if cancelled {
            return Err(Error::Cancelled);
        }
        match result {
            Ok(family) => {
                return Ok(DiscoveredFamily {
                    family,
                    proposals,
                    misses,
                });
            }
            Err(error @ Error::Unsupported(_)) | Err(error @ Error::Invalid(_)) => {
                let Some(cell) = last_cell else {
                    return Err(error);
                };
                misses.push(CandidateMiss {
                    cell,
                    candidate: indices[cell],
                    error: error.clone(),
                });
                indices[cell] += 1;
                if indices[cell] >= portfolios[cell].len() {
                    return Err(unsupported(format!(
                        "automatic candidates exhausted at source cell {cell}: {error}; prior misses={}",
                        misses.len()
                    )));
                }
            }
            Err(error) => return Err(error),
        }
    }
    Err(Error::ResourceIncomplete(
        "discovery candidate attempt allowance".into(),
    ))
}

#[cfg(test)]
mod tests;

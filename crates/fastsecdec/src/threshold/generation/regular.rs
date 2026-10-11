//! Bounded proposals only. Native complete-family issuers retain proof authority.
use super::*;
use crate::threshold::{
    gcad::{AliasRole, DomainOrigin},
    maps::{self, regular::BracketProposal},
    regularization::secant::{self, NormalizedFamily},
};
use std::{collections::BTreeSet, time::Duration};
use symgcad::{
    algebra::Algebra,
    domain::{certify_nonzero, derive_domain, verify_certificate},
};

/// Caller bounds for deterministic rational-cylinder discovery, not a resolver.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryLimits {
    pub max_dyadic_level: u32,
    pub max_endpoint_queries: usize,
    pub max_family_attempts: usize,
    pub max_margin_halvings: u32,
    pub wall_time: Duration,
}
impl Default for DiscoveryLimits {
    fn default() -> Self {
        Self {
            max_dyadic_level: 8,
            max_endpoint_queries: 255,
            max_family_attempts: 64,
            max_margin_halvings: 4,
            wall_time: Duration::from_secs(30),
        }
    }
}
/// Optional regular-family capability (fixed cube or admitted uniform chamber). Unit roles resolve after native input work;
/// the persisted preparation configuration always retains the resolved list.
#[derive(Clone, Debug, Default)]
pub struct RegularFamilyPreparation {
    pub coordinates: Option<Vec<Symbol>>,
    pub limits: secant::Limits,
    pub discovery: DiscoveryLimits,
}
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryProgress {
    pub dyadic_level: u32,
    pub endpoint_queries: usize,
    pub family_attempts: usize,
    pub admitted_charts: Option<usize>,
}

fn native(error: impl std::fmt::Display) -> Error {
    Error::RegularFamily(secant::Error::Native(error.to_string()))
}
fn invalid(message: &str) -> Error {
    Error::RegularFamily(secant::Error::Invalid(message.into()))
}
fn check_settings(settings: &RegularFamilyPreparation) -> Result<()> {
    let d = &settings.discovery;
    if d.max_dyadic_level == 0
        || d.max_dyadic_level > 30
        || d.max_endpoint_queries == 0
        || d.max_family_attempts == 0
        || d.max_margin_halvings > 64
        || d.wall_time.is_zero()
        || settings.limits.geometry.wall_time.is_zero()
        || settings.limits.unit_margin <= Rational::zero()
    {
        return Err(invalid(
            "positive finite regular-family discovery bounds required",
        ));
    }
    Ok(())
}

mod route;
pub(super) use route::{Route, resolve, restore};

struct Search<'a, F> {
    settings: &'a RegularFamilyPreparation,
    started: Instant,
    progress: DiscoveryProgress,
    observer: F,
}
impl<F: FnMut(DiscoveryProgress) -> ControlFlow<()>> Search<'_, F> {
    fn incomplete(&self, reason: &'static str) -> Error {
        Error::DiscoveryIncomplete {
            reason,
            progress: self.progress,
        }
    }
    fn check(&mut self) -> Result<()> {
        if (self.observer)(self.progress).is_break()
            || self
                .settings
                .limits
                .cancellation
                .as_ref()
                .is_some_and(|c| c.load(std::sync::atomic::Ordering::Relaxed))
        {
            return Err(Error::Cancelled);
        }
        if self.started.elapsed() >= self.settings.discovery.wall_time {
            return Err(self.incomplete("overall discovery wall allowance"));
        }
        Ok(())
    }
    fn limits(&mut self, margin: Rational) -> Result<secant::Limits> {
        self.check()?;
        if self.progress.family_attempts >= self.settings.discovery.max_family_attempts {
            return Err(self.incomplete("family attempt allowance"));
        }
        self.progress.family_attempts += 1;
        self.check()?;
        let mut limits = self.settings.limits.clone();
        limits.unit_margin = margin;
        limits.geometry.wall_time = limits.geometry.wall_time.min(
            self.settings
                .discovery
                .wall_time
                .saturating_sub(self.started.elapsed()),
        );
        Ok(limits)
    }
    fn accept(&mut self, family: &NormalizedFamily) -> Result<()> {
        self.progress.admitted_charts = Some(family.charts().len());
        self.check()
    }
}

pub(super) fn discover(
    owner: Arc<gcad::VerifiedDecomposition>,
    settings: &RegularFamilyPreparation,
    observer: impl FnMut(DiscoveryProgress) -> ControlFlow<()>,
) -> Result<NormalizedFamily> {
    check_settings(settings)?;
    let coordinates = settings
        .coordinates
        .as_ref()
        .ok_or_else(|| invalid("unresolved unit roles"))?;
    let request = owner.request();
    let prefix = request.kinematics().runtime_parameters.len();
    let raw_order = &owner.native_result().order;
    let aliases = raw_order
        .iter()
        .map(|name| request.aliases().iter().find(|alias| &alias.name == name))
        .collect::<Option<Vec<_>>>();
    let expected = prefix
        .checked_add(coordinates.len())
        .ok_or_else(|| invalid("regular raw role count overflow"))?;
    let valid = aliases.as_ref().is_some_and(|aliases| {
        aliases.len() == expected
            && aliases[..prefix]
                .iter()
                .all(|a| a.role == AliasRole::RuntimeParameter)
            && aliases[..prefix]
                .iter()
                .map(|a| a.symbol)
                .collect::<BTreeSet<_>>()
                == request
                    .kinematics()
                    .runtime_parameters
                    .iter()
                    .copied()
                    .collect::<BTreeSet<_>>()
            && aliases[prefix..]
                .iter()
                .zip(request.domain().coordinates())
                .all(|(a, s)| a.role == AliasRole::IntegrationCoordinate && a.symbol == *s)
    });
    if !valid {
        return Err(Error::Unsupported(
            "regular-family metadata requires a runtime prefix and original integration-axis order",
        ));
    }
    let uniform = prefix != 0;
    let mut search = Search {
        settings,
        started: Instant::now(),
        progress: Default::default(),
        observer,
    };
    search.check()?;
    if owner.cells().len() == 2 {
        if uniform {
            secant::uniform_single_source_shape(&owner, coordinates)
        } else {
            secant::single_source_shape(&owner, coordinates)
        }
        .map_err(Error::RegularFamily)?;
        let mut margin = settings.limits.unit_margin.clone();
        for _ in 0..=settings.discovery.max_margin_halvings {
            let limits = search.limits(margin.clone())?;
            match attempt(uniform, owner.clone(), coordinates.clone(), None, limits) {
                Ok(family) => {
                    search.accept(&family)?;
                    return Ok(family);
                }
                Err(maps::regular::ProposalError::Miss(_)) => {}
                Err(maps::regular::ProposalError::Failure(error)) => {
                    return Err(Error::RegularFamily(error));
                }
            }
            margin /= Rational::from(2);
        }
        return Err(search.incomplete("single-family certificate candidates exhausted"));
    }
    if owner.cells().len() != 3 {
        return Err(Error::Unsupported(
            "regular-family route requires complete two- or three-cell topology",
        ));
    }
    let middle = if uniform {
        secant::uniform_paired_source(&owner, coordinates)
    } else {
        secant::paired_source(&owner, coordinates)
    }
    .map_err(Error::RegularFamily)?;
    let raw = owner.native_result();
    let axis = middle.axes().len() - 1;
    let algebra = Algebra::new(&raw.order).map_err(native)?;
    let lower = middle.native_axes()[axis]
        .lower
        .as_ref()
        .ok_or_else(|| invalid("missing paired lower section"))?;
    let upper = middle.native_axes()[axis]
        .upper
        .as_ref()
        .ok_or_else(|| invalid("missing paired upper section"))?;
    if lower.polynomial != upper.polynomial || lower == upper {
        return Err(Error::Unsupported(
            "paired discovery requires distinct sections of the same native polynomial",
        ));
    }
    let polynomial = algebra
        .parse(&raw.polynomials[lower.polynomial])
        .map_err(native)?;
    if polynomial.nterms() > settings.limits.geometry.max_terms
        || (0..raw.order.len())
            .any(|i| polynomial.degree(i) as usize > settings.limits.geometry.max_degree)
    {
        return Err(search.incomplete("discovery polynomial allowance"));
    }
    let constraints = raw
        .normalized_constraints
        .iter()
        .map(|p| algebra.parse(p).map_err(native))
        .collect::<Result<Vec<_>>>()?;
    let domain = derive_domain(&algebra, &constraints, &[]).map_err(native)?;
    if domain.empty {
        return Err(invalid("empty original discovery domain"));
    }
    let mut endpoints = BTreeMap::new();
    let mut tried = BTreeSet::new();
    for level in 1..=settings.discovery.max_dyadic_level {
        search.progress.dyadic_level = level;
        let denominator = 1u32
            .checked_shl(level)
            .ok_or_else(|| invalid("dyadic level overflow"))?;
        for numerator in (1..denominator).step_by(2) {
            search.check()?;
            if search.progress.endpoint_queries >= settings.discovery.max_endpoint_queries {
                return Err(search.incomplete("endpoint certificate query allowance"));
            }
            let q = Rational::from((i64::from(numerator), i64::from(denominator)));
            crate::threshold::maps::regular::Budget::new(settings.limits.geometry.clone())
                .rational(&q)
                .map_err(secant::Error::from)
                .map_err(Error::RegularFamily)?;
            let target = polynomial.replace(axis, &q);
            search.progress.endpoint_queries += 1;
            if let Some(certificate) = certify_nonzero(
                &algebra,
                &target,
                &domain,
                &settings.limits.geometry.certificates,
            )
            .map_err(native)?
            {
                verify_certificate(&algebra, &target, &constraints, &[], &certificate)
                    .map_err(native)?;
                endpoints.insert(q, certificate.sign);
            }
            search.check()?;
        }
        let ordered = endpoints.iter().collect::<Vec<_>>();
        let candidates = ordered
            .windows(2)
            .filter(|w| w[0].1 != w[1].1)
            .map(|w| (w[0].0.clone(), w[1].0.clone()))
            .collect::<Vec<_>>();
        // The verified topology owns exactly two sections. Extra transitions
        // cannot be silently discarded or assigned by approximate root order.
        if candidates.len() != 2 || candidates[0].1 >= candidates[1].0 {
            continue;
        }
        let mut margin = settings.limits.unit_margin.clone();
        for _ in 0..=settings.discovery.max_margin_halvings {
            let key = (candidates.clone(), margin.clone());
            if tried.insert(key) {
                let limits = search.limits(margin.clone())?;
                let proposals = [0, 1].map(|i| BracketProposal {
                    lower: candidates[i].0.clone(),
                    upper: candidates[i].1.clone(),
                    derivative_margin: margin.clone(),
                });
                match attempt(
                    uniform,
                    owner.clone(),
                    coordinates.clone(),
                    Some(proposals),
                    limits,
                ) {
                    Ok(family) => {
                        search.accept(&family)?;
                        return Ok(family);
                    }
                    Err(maps::regular::ProposalError::Miss(_)) => {}
                    Err(maps::regular::ProposalError::Failure(error)) => {
                        return Err(Error::RegularFamily(error));
                    }
                }
            }
            margin /= Rational::from(2);
        }
    }
    Err(search.incomplete("paired cylinder candidates exhausted"))
}

fn attempt(
    uniform: bool,
    owner: Arc<gcad::VerifiedDecomposition>,
    coordinates: Vec<Symbol>,
    proposals: Option<[BracketProposal; 2]>,
    limits: secant::Limits,
) -> std::result::Result<NormalizedFamily, maps::regular::ProposalError<secant::Error>> {
    if uniform {
        secant::attempt_parameter_affine_shifted(owner, coordinates, proposals, limits)
    } else {
        secant::attempt_affine_shifted(owner, coordinates, proposals, limits)
    }
}

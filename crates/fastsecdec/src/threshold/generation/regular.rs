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
/// Optional fixed-cube capability. Unit roles resolve after native input work;
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

/// Resolve only the newly enabled multidimensional path. Historical one-axis
/// rational preparation has no regular-family configuration or new symbols.
pub(super) fn resolve(options: &mut PreparationOptions, request: &gcad::GcadRequest) -> Result<()> {
    let n = request.domain().coordinates().len();
    if n == 1 {
        options.regular_family = None;
        return Ok(());
    }
    let settings = options.regular_family.as_mut().ok_or(Error::Unsupported(
        "multidimensional threshold preparation requires the regular-family capability",
    ))?;
    check_settings(settings)?;
    if n == 0
        || !matches!(request.domain().origin(), DomainOrigin::NativeUnitCube)
        || !request.kinematics().runtime_parameters.is_empty()
        || !options.fixed_fiber.is_empty()
    {
        return Err(Error::Unsupported(
            "automatic regular families require a fixed native cube with no runtime/fixed-fiber axes",
        ));
    }
    let original = maps::coordinate_symbols(request);
    let mut occupied = original.clone();
    if occupied.contains(&options.unit) || !options.unit.is_exportable() {
        return Err(invalid(
            "regular unit collides with source roles or carries a native hook",
        ));
    }
    if settings.coordinates.is_none() {
        occupied.insert(options.unit);
        let mut coordinates = Vec::with_capacity(n);
        for axis in 0..n - 1 {
            let mut issued = None;
            for attempt in 0..1024usize {
                let name = format!("{}_prefix_{axis}_{attempt}", options.unit.get_name());
                let candidate =
                    Symbol::parse(&name, "fastsecdec_regular_family").map_err(native)?;
                if !candidate.is_exportable() {
                    return Err(invalid(
                        "regular coordinate carries an existing native hook",
                    ));
                }
                if occupied.insert(candidate) {
                    issued = Some(candidate);
                    break;
                }
            }
            coordinates.push(issued.ok_or_else(|| invalid("fresh regular coordinate allowance"))?);
        }
        coordinates.push(options.unit);
        settings.coordinates = Some(coordinates);
    }
    let coordinates = settings.coordinates.as_ref().expect("resolved above");
    if coordinates.len() != n
        || coordinates.last() != Some(&options.unit)
        || coordinates.iter().copied().collect::<BTreeSet<_>>().len() != n
        || coordinates
            .iter()
            .any(|s| original.contains(s) || !s.is_exportable())
    {
        return Err(invalid(
            "regular coordinates must be distinct fresh original-order roles",
        ));
    }
    Ok(())
}

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
    if owner
        .native_result()
        .order
        .iter()
        .map(|name| request.aliases().iter().find(|alias| &alias.name == name))
        .zip(request.domain().coordinates())
        .any(|(alias, symbol)| {
            alias.is_none_or(|alias| {
                alias.role != AliasRole::IntegrationCoordinate || alias.symbol != *symbol
            })
        })
        || owner.native_result().order.len() != coordinates.len()
    {
        return Err(Error::Unsupported(
            "regular-family metadata requires original integration-axis order",
        ));
    }
    let mut search = Search {
        settings,
        started: Instant::now(),
        progress: Default::default(),
        observer,
    };
    search.check()?;
    if owner.cells().len() == 2 {
        secant::single_source_shape(&owner, coordinates).map_err(Error::RegularFamily)?;
        let mut margin = settings.limits.unit_margin.clone();
        for _ in 0..=settings.discovery.max_margin_halvings {
            let limits = search.limits(margin.clone())?;
            match secant::attempt_affine_shifted(owner.clone(), coordinates.clone(), None, limits) {
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
    let middle = secant::paired_source(&owner, coordinates).map_err(Error::RegularFamily)?;
    let raw = owner.native_result();
    let axis = coordinates.len() - 1;
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
                match secant::attempt_affine_shifted(
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

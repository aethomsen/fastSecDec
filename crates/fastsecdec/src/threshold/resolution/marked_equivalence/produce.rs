use super::super::{Budget, Error};
use super::*;
use symbolica::{domains::integer::gcd_unsigned, poly::factor::Factorize};
type Result<T> = std::result::Result<T, Error>;
enum Built {
    Reduced(Box<PrincipalPowerPresentation>),
    Unchanged(PrincipalPowerUnchangedReason),
}
/// Discover only native polynomial principal powers; quotient-only powers may
/// remain unchanged. Caller owns hard native factoring/membership resource caps.
pub fn produce_principal_power_presentation(
    history: Arc<ResolutionHistory>,
    source: Arc<MarkedIdeal>,
    b: &mut Budget,
) -> Result<PrincipalPowerOutcome> {
    let local = history.ledger().frame().local();
    if source.ideal().ring() != local.ring() {
        return Err(Error::Invalid("principal power source/history ring"));
    }
    for p in source.ideal().generators() {
        local.supports(p)?;
    }
    let mut progress = PrincipalPowerProgress::default();
    let result = build(&history, &source, b, &mut progress);
    progress.operations = b.operations();
    progress.ideal_slots = b.ideal_slots();
    Ok(match result {
        Ok(Built::Reduced(mut r)) => {
            r.progress = progress;
            PrincipalPowerOutcome::Reduced(Arc::new(*r))
        }
        Ok(Built::Unchanged(reason)) => PrincipalPowerOutcome::Unchanged(PrincipalPowerUnchanged {
            source,
            history,
            reason,
            progress,
        }),
        Err(Error::ResourceIncomplete(reason)) => PrincipalPowerOutcome::Incomplete {
            source,
            history,
            reason,
            progress,
        },
        Err(e) => return Err(e),
    })
}
fn build(
    history: &Arc<ResolutionHistory>,
    source: &Arc<MarkedIdeal>,
    b: &mut Budget,
    p: &mut PrincipalPowerProgress,
) -> Result<Built> {
    p.stage = "source admission";
    let local = history.ledger().frame().local();
    let generators = source.ideal().generators();
    b.count(generators.len())?;
    if source.mark() > b.limits.max_mark {
        return Err(Error::ResourceIncomplete("marked-ideal weight"));
    }
    for g in generators {
        b.poly(g)?;
    }
    if generators.is_empty() {
        return Ok(Built::Unchanged(PrincipalPowerUnchangedReason::ZeroIdeal));
    }
    if generators.len() != 1 {
        return Ok(Built::Unchanged(
            PrincipalPowerUnchangedReason::MultipleGenerators,
        ));
    }
    let original = &generators[0];
    if local.zero(original, b)? {
        return Ok(Built::Unchanged(PrincipalPowerUnchangedReason::ZeroIdeal));
    }
    p.stage = "native square-free factorization";
    b.charge(1)?;
    let factors = original.square_free_factorization();
    p.native_factor_count = factors.len();
    b.count(factors.len())?;
    b.reserve_slots(factors.len())?;
    if factors.is_empty() {
        return Err(Error::Invalid("empty native principal factorization"));
    }
    let mut reconstructed = local.ring().one();
    let mut common = source.mark();
    let mut nonunits = 0usize;
    for (f, n) in factors {
        b.poly(&f)?;
        local.supports(&f)?;
        if n == 0 || f.is_zero() {
            return Err(Error::Invalid("native principal factor multiplicity"));
        }
        let factor_power = b.power(&f, n)?;
        reconstructed = b.mul(&reconstructed, &factor_power)?;
        p.stage = "original-localization factor unit";
        let augmented = local
            .ideal()
            .sum(&Ideal::new(local.ring().clone(), vec![f.clone()], b)?, b)?;
        let unit = augmented.contains(&local.ring().one(), local.unit_relations(), b)?;
        if !unit {
            nonunits = nonunits
                .checked_add(1)
                .ok_or(Error::ResourceIncomplete("principal factor count"))?;
            common = usize::try_from(gcd_unsigned(
                u64::try_from(common)
                    .map_err(|_| Error::ResourceIncomplete("principal mark conversion"))?,
                u64::try_from(n)
                    .map_err(|_| Error::ResourceIncomplete("principal multiplicity conversion"))?,
            ))
            .map_err(|_| Error::ResourceIncomplete("principal gcd conversion"))?;
        }
        p.checked_factors.push(PrincipalPowerFactor {
            polynomial: f,
            multiplicity: n,
            unit_ideal: unit.then_some(augmented),
        });
    }
    p.stage = "native factorization reconstruction";
    if reconstructed != *original || !local.zero(&(&reconstructed - original), b)? {
        return Err(Error::Invalid("native principal factor reconstruction"));
    }
    p.recombination_checked = true;
    p.common_power = Some(common);
    if nonunits == 0 {
        return Ok(Built::Unchanged(PrincipalPowerUnchangedReason::UnitIdeal));
    }
    if common <= 1 {
        return Ok(Built::Unchanged(
            PrincipalPowerUnchangedReason::NoCommonPower,
        ));
    }
    let mut unit = local.ring().one();
    let mut base = local.ring().one();
    p.stage = "principal power candidate";
    for factor in &p.checked_factors {
        if factor.is_unit() {
            let power = b.power(&factor.polynomial, factor.multiplicity)?;
            unit = b.mul(&unit, &power)?;
        } else {
            if !factor.multiplicity.is_multiple_of(common) {
                return Err(Error::Invalid("principal power exponent divisibility"));
            }
            let power = b.power(&factor.polynomial, factor.multiplicity / common)?;
            base = b.mul(&base, &power)?;
        }
    }
    if !source.mark().is_multiple_of(common) {
        return Err(Error::Invalid("principal power mark divisibility"));
    }
    let base_power = b.power(&base, common)?;
    let product = b.mul(&unit, &base_power)?;
    if !local.zero(&(original - &product), b)? {
        return Err(Error::Invalid("principal power localized reconstruction"));
    }
    let reduced = Arc::new(MarkedIdeal::new(
        Ideal::new(local.ring().clone(), vec![base.clone()], b)?,
        source.mark() / common,
        b,
    )?);
    p.stage = "complete";
    Ok(Built::Reduced(Box::new(PrincipalPowerPresentation {
        source: source.clone(),
        history: history.clone(),
        reduced,
        unit,
        base,
        power: common,
        progress: PrincipalPowerProgress::default(),
    })))
}

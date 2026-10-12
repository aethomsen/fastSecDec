//! Shared capability decision for fresh preparation and raw-evidence recovery.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Route {
    LegacyFiber,
    FixedCube,
    Uniform,
    ResolvedProjective,
    PositiveProjective,
}

fn classify(options: &PreparationOptions, request: &Request) -> Result<Route> {
    let Request::Affine(request) = request else {
        // The typed request already binds a complete original order/anchor.
        // Recovery configuration intentionally has default solver fields.
        if request.dimensions() != 2
            || options.threshold.kinematics != gcad::GcadKinematics::default()
            || !options.fixed_fiber.is_empty()
        {
            return Err(Error::Unsupported(
                "positive automatic discovery requires two fixed projective axes",
            ));
        }
        return Ok(Route::PositiveProjective);
    };
    let n = request.domain().coordinates().len();
    let parameters = &request.kinematics().runtime_parameters;
    // Preserve the original explicit fixed-fiber path, including its later
    // native admission of complete bindings. Dimension does not certify roots.
    if n == 1 && (parameters.is_empty() || !options.fixed_fiber.is_empty()) {
        return Ok(Route::LegacyFiber);
    }
    if n == 0 || !options.fixed_fiber.is_empty() {
        return Err(Error::Unsupported(
            "regular families require integration axes and no fixed-fiber specialization",
        ));
    }
    if parameters.is_empty() {
        return match request.domain().origin() {
            DomainOrigin::NativeUnitCube => Ok(Route::FixedCube),
            DomainOrigin::AffineProjective { .. } if n >= 2 => Ok(Route::ResolvedProjective),
            _ => Err(Error::Unsupported(
                "fixed regular preparation requires a native cube or a supported resolved projective family",
            )),
        };
    }
    match request.domain().origin() {
        DomainOrigin::NativeUnitCube => Ok(Route::Uniform),
        DomainOrigin::AffineProjective { .. } if n == 1 => Ok(Route::Uniform),
        _ => Err(Error::Unsupported(
            "uniform regular preparation requires a native cube or one-axis affine projective source",
        )),
    }
}

pub(crate) fn resolve(options: &mut PreparationOptions, request: &Request) -> Result<Route> {
    let route = classify(options, request)?;
    if route == Route::LegacyFiber {
        options.regular_family = None;
        return Ok(route);
    }
    let n = request.dimensions();
    let settings = options.regular_family.as_mut().ok_or(Error::Unsupported(
        "automatic regular preparation requires the regular-family capability",
    ))?;
    check_settings(settings)?;
    let original = request.occupied();
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
    Ok(route)
}

pub(crate) fn restore(
    options: &mut PreparationOptions,
    request: &Request,
    stored: Route,
) -> Result<()> {
    if classify(options, request)? != stored
        || options.regular_family.is_some() != (stored != Route::LegacyFiber)
    {
        return Err(Error::Association(
            "regular-family capability/request association",
        ));
    }
    resolve(options, request)?;
    Ok(())
}

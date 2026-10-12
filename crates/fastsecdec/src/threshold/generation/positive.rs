//! Explicit positive-anchor selection and automatic complete-family finishing.
use super::*;

pub(super) fn selected(options: &PreparationOptions) -> bool {
    options.threshold.solver.homogeneous_anchor.is_some()
}
pub(super) fn anchor(options: &PreparationOptions, input: &ParametricIntegrand) -> Result<usize> {
    let solver = &options.threshold.solver;
    if options.threshold.kinematics != gcad::GcadKinematics::default()
        || !options.fixed_fiber.is_empty()
    {
        return Err(Error::Unsupported(
            "positive-projective generation requires fixed original kinematics",
        ));
    }
    let anchor = solver
        .homogeneous_anchor
        .as_ref()
        .ok_or(Error::Unsupported(
            "positive route requires an explicit original anchor",
        ))?;
    if solver.ordering != "given"
        || solver.order.len() != input.parameters().len()
        || solver.order.first() != Some(anchor)
    {
        return Err(Error::Unsupported(
            "positive route requires an explicit complete original order with anchor first",
        ));
    }
    (0..input.parameters().len())
        .find(|i| format!("x{i}") == *anchor)
        .ok_or(Error::Unsupported(
            "positive anchor is not an original source alias",
        ))
}

pub(super) fn plan<F: FnMut(&Progress) -> ControlFlow<()>>(
    owner: Arc<gcad::PositiveProjectiveQuotient>,
    options: &PreparationOptions,
    vectors: &Path,
    reporter: &mut Reporter<'_, F>,
) -> Result<(ThresholdCompilationPlan, usize)> {
    use regularization::projective_pair_family as pair;
    let settings = options
        .regular_family
        .as_ref()
        .ok_or(Error::Association("missing positive-family capability"))?;
    let coordinates = settings
        .coordinates
        .as_ref()
        .ok_or(Error::Association("unresolved positive chart roles"))?
        .clone();
    let mut limits = pair::Limits {
        algebra: settings.limits.geometry.clone(),
        sectors: options.generation.decomposition.clone(),
        prefactors: settings.limits.density,
        max_charts: options.regularization.max_charts,
        max_field_degree: settings.limits.geometry.max_degree,
    };
    limits.algebra.wall_time = limits.algebra.wall_time.min(settings.discovery.wall_time);
    // The existing resource policy bounds this initial proposal portfolio. This
    // does not increase any bound or widen the quadratic automatic capability.
    let discovery = pair::discovery::DiscoveryOptions {
        max_subdivision_depth: settings.discovery.max_dyadic_level.min(12),
        max_attempts: settings.discovery.max_family_attempts,
        wall_time: settings.discovery.wall_time,
    };
    let result = pair::discovery::discover(
        owner,
        coordinates,
        limits,
        discovery,
        options.compilation.native(),
        |p| {
            let completed = match p {
                pair::Progress::Cell(n)
                | pair::Progress::Chart(n)
                | pair::Progress::Continue(n) => n,
                pair::Progress::Half { cell, .. } => cell,
            };
            if settings
                .limits
                .cancellation
                .as_ref()
                .is_some_and(|c| c.load(std::sync::atomic::Ordering::Relaxed))
                || reporter.poll(completed, None).is_err()
            {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        },
    )?;
    let charts = result.family.charts().len();
    reporter.regular_family = Some(DiscoveryProgress {
        family_attempts: result.misses.len() + 1,
        admitted_charts: Some(charts),
        ..Default::default()
    });
    reporter.stage(Stage::Continue, 0, Some(charts))?;
    let continued = result
        .family
        .continue_symbolically(&options.generation, |p| {
            let n = match p {
                pair::Progress::Continue(n) => n,
                _ => 0,
            };
            if reporter.poll(n, Some(charts)).is_err() {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })?;
    reporter.stage(Stage::StageVectors, 0, Some(charts))?;
    let plan = ThresholdCompilationPlan::prepare_projective_pair_family(
        &continued,
        vectors,
        options.generation.max_order,
        options.precision.clone(),
        options.compilation,
    )?;
    Ok((plan, charts))
}

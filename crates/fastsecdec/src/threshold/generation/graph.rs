//! Fixed native graph preparation before any numerical family arithmetic.
use super::*;
use represented::graph::{ExactRepresentedGraphInput, GraphPoint};

pub(super) fn admission(options: &PreparationOptions) -> Result<()> {
    if options.represented.is_none() {
        return Err(Error::Unsupported(
            "graph preparation requires explicit represented-value meaning and limits",
        ));
    }
    if options.threshold.kinematics != gcad::GcadKinematics::default()
        || !options.fixed_fiber.is_empty()
    {
        return Err(Error::Unsupported(
            "graph preparation requires fixed original kinematics and scalar bindings",
        ));
    }
    Ok(())
}

pub(super) fn request_options(
    options: &PreparationOptions,
    request: &gcad::GcadRequest,
) -> Result<()> {
    let owner = request
        .preparametric_graph_input()
        .ok_or(Error::Association(
            "graph replay requires pre-parametric graph provenance",
        ))?;
    if options.represented != Some((owner.meaning(), owner.limits()))
        || options.graph_payloads != owner.payload_policy()
    {
        return Err(Error::Association("graph conversion policy/configuration"));
    }
    Ok(())
}

/// Convert the original represented scalar point before native family algebra,
/// then use the existing verified preparation/publication path. The original
/// graph, precision and conversion policy enter the issued source identity.
/// Input-stage timing includes conversion and native parameterization. Limits
/// bound conversion and transport, not parameterization/decoded process RSS.
pub fn prepare_graph(
    original: Arc<GraphPoint>,
    options: PreparationOptions,
    root: &Path,
    mut observer: impl FnMut(&Progress) -> ControlFlow<()>,
) -> Result<PreparedThreshold> {
    admission(&options)?;
    prepare_with(options, root, &mut observer, |options, reporter| {
        let (meaning, limits) = options.represented.expect("graph admission checked");
        let owner = ExactRepresentedGraphInput::prepare_with_payloads(
            original,
            meaning,
            limits,
            options.graph_payloads,
            |p| {
                reporter.graph_input = Some(p);
                if reporter.poll(p.converted_literals, None).is_ok() {
                    ControlFlow::Continue(())
                } else {
                    ControlFlow::Break(())
                }
            },
        )
        .map_err(|error| match error {
            represented::Error::Cancelled => Error::Cancelled,
            error => Error::Geometry(gcad::GcadError::Represented(error)),
        })?;
        if positive::selected(options) {
            let anchor = positive::anchor(options, owner.exact())?;
            return Ok(Request::Positive(Arc::new(
                gcad::PositiveProjectiveRequest::graph(
                    Arc::new(owner),
                    anchor,
                    options.threshold.solver.clone(),
                    options.threshold.gcad_limits.clone(),
                )?,
            )));
        }
        let eliminated =
            owner
                .exact()
                .parameters()
                .len()
                .checked_sub(1)
                .ok_or(Error::Unsupported(
                    "graph preparation has no projective coordinate",
                ))?;
        Ok(Request::Affine(Arc::new(
            gcad::GcadRequest::preparametric_projective(
                Arc::new(owner),
                eliminated,
                options.threshold.solver.clone(),
                options.threshold.gcad_limits.clone(),
            )?,
        )))
    })
}

/// Rebuild the represented original point and reverify saved raw evidence,
/// without solving. The caller supplies the original native graph point and
/// an independent conversion cap; record bytes cannot create that authority.
/// Existing integration artifacts do not require this original owner.
#[allow(clippy::too_many_arguments)]
pub fn resume_graph_evidence(
    original: Arc<GraphPoint>,
    conversion_cap: represented::Limits,
    root: &Path,
    checkpoint: &PreparationCheckpoint,
    work_directory: &str,
    expected_source: &str,
    maximum_bytes: u64,
    mut observer: impl FnMut(&Progress) -> ControlFlow<()>,
) -> Result<PreparedThreshold> {
    recover(
        root,
        checkpoint,
        work_directory,
        expected_source,
        None,
        maximum_bytes,
        Some((OriginalPoint::Graph(original), conversion_cap)),
        &mut observer,
    )
}

/// Complete-receipt replay additionally requires the previously issued prepared
/// identity. Missing/wrong original input is a refusal, never an exact fallback.
#[allow(clippy::too_many_arguments)]
pub fn resume_graph(
    original: Arc<GraphPoint>,
    conversion_cap: represented::Limits,
    root: &Path,
    prior: &PreparationReceipt,
    work_directory: &str,
    expected_source: &str,
    expected_prepared: &str,
    maximum_bytes: u64,
    mut observer: impl FnMut(&Progress) -> ControlFlow<()>,
) -> Result<PreparedThreshold> {
    if prior.publication.source_identity != expected_source
        || prior.publication.prepared_identity != expected_prepared
        || prior.strategy != ThresholdStrategy::GcadFirst
    {
        return Err(Error::Association(
            "resume source, recipe or prepared identity",
        ));
    }
    recover(
        root,
        &prior.checkpoint()?,
        work_directory,
        expected_source,
        Some(expected_prepared),
        maximum_bytes,
        Some((OriginalPoint::Graph(original), conversion_cap)),
        &mut observer,
    )
}

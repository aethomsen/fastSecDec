use super::*;
use crate::kernel::threshold_metadata::MapDescriptor;

fn triangle() -> (Arc<ParametricIntegrand>, PreparationOptions) {
    let (x, y, z, eps, unit) = symbol!(
        "projective_adapter::x",
        "projective_adapter::y",
        "projective_adapter::z",
        "projective_adapter::eps",
        "projective_adapter::unit"
    );
    let u = Atom::var(x) + Atom::var(y) + Atom::var(z);
    let f = &u * &u - Atom::num(5) * Atom::var(x) * Atom::var(y);
    let input = ParametricIntegrand::new(
        vec![x, y, z],
        eps,
        ParametricDomain::ProjectiveSimplex,
        vec![ParametricTerm::new(
            -Atom::one(),
            vec![Atom::zero(); 3],
            vec![
                PolynomialFactor::new(
                    u,
                    -Atom::one() + Atom::num(2) * Atom::var(eps),
                    FactorRole::Singularity,
                )
                .with_semantics(FactorSemantics::Positive),
                PolynomialFactor::new(f, -Atom::one() - Atom::var(eps), FactorRole::Singularity)
                    .with_semantics(FactorSemantics::Causal),
            ],
        )],
    )
    .unwrap();
    let mut options = PreparationOptions::new(unit);
    options.regular_family = Some(RegularFamilyPreparation::default());
    options.threshold.solver.order = vec!["v1".into(), "v0".into()];
    options.threshold.gcad_limits.wall_time_secs = 30.;
    options.threshold.gcad_limits.memory_mib = 1024;
    options.generation.max_order = 0;
    (Arc::new(input), options)
}

#[test]
fn resolved_projective_preparation_replay_and_selective_artifact() {
    let directory = tempfile::tempdir().unwrap();
    let (input, options) = triangle();
    let first = prepare(input, options, directory.path(), |_| {
        ControlFlow::Continue(())
    })
    .unwrap()
    .receipt;
    assert_eq!(
        (first.original_dimensions, first.integration_dimensions),
        (3, 2)
    );
    assert_eq!(first.cells, 4);
    assert_eq!(first.endpoint_charts, 18);
    let (_, route) =
        configuration::read(directory.path(), &first.configuration, 32 * 1024 * 1024).unwrap();
    assert_eq!(route, regular::Route::ResolvedProjective);
    let (archive, catalogue) = compile_archive(directory.path(), &first);
    let mut original =
        KernelSet::from_bytes_with_options(&archive, KernelLoadOptions { validate: true }).unwrap();
    assert_eq!(
        original.content_id(),
        catalogue
            .recipe(crate::kernel::ProgramRecipe::ThresholdV1)
            .unwrap()
            .content_id
    );
    let mut stages = Vec::new();
    let replay = resume(
        directory.path(),
        &first,
        "replay-vectors",
        &first.publication.source_identity,
        &first.publication.prepared_identity,
        32 * 1024 * 1024,
        |p| {
            stages.push(p.stage);
            ControlFlow::Continue(())
        },
    )
    .unwrap()
    .receipt;
    assert!(!stages.contains(&Stage::Solve));
    assert!(stages.contains(&Stage::Verify));
    let mut restored = compile(directory.path(), &replay);
    assert_eq!(original.content_id(), restored.content_id());
    let metadata = original.threshold_metadata().unwrap();
    assert!(metadata.full_original_scope().unwrap());
    assert_eq!(metadata.lineage().cells.len(), 4);
    assert!(
        metadata
            .lineage()
            .endpoint_charts
            .iter()
            .all(|c| matches!(c.map, MapDescriptor::ResolvedEndpointV1 { .. }))
    );
    for point in [[0.23, 0.37], [0.79, 0.61], [0.01, 0.99]] {
        for (a, b) in original
            .sectors_mut()
            .iter_mut()
            .zip(restored.sectors_mut())
        {
            let mut av = vec![0.; a.output_count()];
            let mut bv = vec![0.; b.output_count()];
            a.evaluate(&point, &mut av).unwrap();
            b.evaluate(&point, &mut bv).unwrap();
            assert_eq!(av, bv);
        }
    }
    let mut reader = crate::kernel::indexed::ProgramArchiveReader::from_reader(
        Cursor::new(archive),
        KernelLoadOptions { validate: true },
    )
    .unwrap();
    let selected = reader
        .select(crate::kernel::ProgramRecipe::ThresholdV1)
        .unwrap()
        .load_sector(0)
        .unwrap();
    assert_eq!(selected.sectors().len(), 1);
    assert!(
        !selected
            .threshold_metadata()
            .unwrap()
            .full_original_scope()
            .unwrap()
    );
}

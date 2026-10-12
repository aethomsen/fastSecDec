use super::*;
use crate::{
    generation::GenerationOptions,
    kernel::{CompilationSettings, EvaluatorBackend, KernelLoadOptions},
    parametric::{
        FactorRole, FactorSemantics, ParametricDomain, ParametricIntegrand, ParametricTerm,
        PolynomialFactor,
    },
    threshold::{
        gcad::{GcadKinematics, GcadRequest, SolverOptions},
        projective::AffineProjectivePreparation,
        regularization::projective_family::ResolvedProjectiveFamily,
    },
};
use std::ops::ControlFlow;
use symbolica::symbol;

#[test]
fn legacy_v7_resolved_identity_and_restore_keep_empty_numerator_wire() {
    let (x, y, z, eps, u, v) = symbol!(
        "positive_publication_legacy::x",
        "positive_publication_legacy::y",
        "positive_publication_legacy::z",
        "positive_publication_legacy::eps",
        "positive_publication_legacy::u",
        "positive_publication_legacy::v"
    );
    let first = Atom::var(x) + Atom::var(y) + Atom::var(z);
    let second = &first * &first - Atom::num(5) * Atom::var(x) * Atom::var(z);
    let input = ParametricIntegrand::new(
        vec![x, y, z],
        eps,
        ParametricDomain::ProjectiveSimplex,
        vec![ParametricTerm::new(
            -Atom::one(),
            vec![Atom::zero(); 3],
            vec![
                PolynomialFactor::new(
                    first,
                    -Atom::one() + Atom::num(2) * Atom::var(eps),
                    FactorRole::Singularity,
                )
                .with_semantics(FactorSemantics::Positive),
                PolynomialFactor::new(
                    second,
                    -Atom::one() - Atom::var(eps),
                    FactorRole::Singularity,
                )
                .with_semantics(FactorSemantics::Causal),
            ],
        )],
    )
    .unwrap();
    let source = Arc::new(
        GcadRequest::projective(
            AffineProjectivePreparation::eliminate(&input, 1).unwrap(),
            GcadKinematics::default(),
            SolverOptions {
                order: vec!["v1".into(), "v0".into()],
                ..Default::default()
            },
            symgcad::Limits {
                workers: 1,
                wall_time_secs: 10.,
                memory_mib: 1024,
                ..Default::default()
            },
        )
        .unwrap()
        .solve_verified()
        .unwrap(),
    );
    let family = ResolvedProjectiveFamily::admit(source, vec![u, v], Default::default(), |_| {
        ControlFlow::Continue(())
    })
    .unwrap();
    let continued = family
        .continue_symbolically(
            &GenerationOptions {
                max_order: 0,
                ..Default::default()
            },
            |_| ControlFlow::Continue(()),
        )
        .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let compiled = KernelSet::compile_threshold_projective_family(
        &continued,
        directory.path(),
        0,
        Default::default(),
        CompilationSettings {
            backend: EvaluatorBackend::Eager,
            ..Default::default()
        },
    )
    .unwrap();
    let metadata = compiled.threshold_metadata().unwrap();
    let semantic: serde_json::Value = serde_json::from_str(&metadata.semantic).unwrap();
    assert_eq!(semantic["projection_version"], 7);
    assert_eq!(semantic["recipe"], "resolved_projective_endpoint_v1");
    assert!(semantic["native_values"].is_array());
    assert!(semantic.get("numerator_transports").is_none());
    for chart in &metadata.lineage().endpoint_charts {
        let endpoint = chart.map.endpoint().unwrap();
        assert!(endpoint.terms.iter().all(|t| t.numerators.is_empty()));
    }
    let old_wire = serde_json::to_string(metadata.lineage()).unwrap();
    assert!(!old_wire.contains("\"numerators\""));
    let bytes = compiled.to_bytes().unwrap();
    let restored =
        KernelSet::from_bytes_with_options(&bytes, KernelLoadOptions { validate: true }).unwrap();
    assert_eq!(compiled.content_id(), restored.content_id());
    assert_eq!(
        metadata.semantic,
        restored.threshold_metadata().unwrap().semantic
    );
    assert_eq!(
        old_wire,
        serde_json::to_string(restored.threshold_metadata().unwrap().lineage()).unwrap()
    );
    for i in 0..compiled.sectors().len() {
        let mut expected = vec![0.; compiled.components().len()];
        let mut actual = vec![0.; restored.components().len()];
        compiled
            .evaluation_context(i, Default::default())
            .unwrap()
            .evaluate_weighted(&[0.23, 0.37], 1., &mut expected)
            .unwrap();
        restored
            .evaluation_context(i, Default::default())
            .unwrap()
            .evaluate_weighted(&[0.23, 0.37], 1., &mut actual)
            .unwrap();
        assert_eq!(actual, expected);
    }
}

fn paired_plan(root: &std::path::Path, fresh: bool) -> crate::kernel::ThresholdCompilationPlan {
    use crate::threshold::{
        gcad::PositiveProjectiveQuotient,
        regularization::projective_pair_family::{ResolvedProjectivePairFamily, tests as fixture},
    };
    let (x, eps) = symbol!("pair_test::x", "pair_test::eps");
    let owner = fixture::fixture(
        Some(((Atom::one() + Atom::num(2) * Atom::i()) * Atom::var(x), 1)),
        Atom::one() + Atom::var(eps),
    );
    let owner = if fresh {
        // The second invocation performs a fresh genuine solve. Its native
        // statistics and resource settings are provenance, not arithmetic.
        let mut raw = owner.original_result().as_ref().clone();
        raw.statistics.total_seconds += 123.;
        raw.homogeneous
            .as_mut()
            .unwrap()
            .reduced
            .statistics
            .total_seconds += 17.;
        raw.problem.limits.wall_time_secs += 1.;
        raw.homogeneous
            .as_mut()
            .unwrap()
            .reduced
            .problem
            .limits
            .wall_time_secs += 1.;
        Arc::new(
            PositiveProjectiveQuotient::verify(owner.preparation().clone(), Arc::new(raw)).unwrap(),
        )
    } else {
        owner
    };
    let units = if fresh {
        for i in 0..17 {
            Symbol::parse(format!("positive_fresh::unrelated_{i}"), "").unwrap();
        }
        vec![symbol!("positive_fresh::a"), symbol!("positive_fresh::b")]
    } else {
        fixture::coordinates()
    };
    let limits = fixture::limits();
    let proposals = fixture::fixtures::proposals(&owner, &units, &limits).unwrap();
    let family = ResolvedProjectivePairFamily::admit(
        owner,
        units,
        proposals,
        limits,
        Default::default(),
        |_| ControlFlow::Continue(()),
    )
    .unwrap();
    let source = Arc::downgrade(family.owner());
    let raw = Arc::downgrade(family.owner().original_result());
    let helpers = family
        .callback_scope()
        .owners()
        .map(|(_, p)| Arc::downgrade(p))
        .collect::<Vec<_>>();
    let continued = family
        .continue_symbolically(
            &GenerationOptions {
                max_order: 0,
                ..Default::default()
            },
            |_| ControlFlow::Continue(()),
        )
        .unwrap();
    let plan = crate::kernel::ThresholdCompilationPlan::prepare_projective_pair_family(
        &continued,
        root,
        0,
        Default::default(),
        CompilationSettings {
            backend: EvaluatorBackend::Eager,
            ..Default::default()
        },
    )
    .unwrap();
    drop(continued);
    drop(family);
    assert!(
        source.upgrade().is_none(),
        "plan must not retain the source quotient"
    );
    assert!(
        raw.upgrade().is_none(),
        "plan must not retain complete native geometry"
    );
    assert!(
        helpers.iter().all(|h| h.upgrade().is_none()),
        "native Scope is detached through its existing codec"
    );
    plan
}

fn selected_observation(bytes: Vec<u8>) -> serde_json::Value {
    use crate::kernel::{ProgramRecipe, indexed::ProgramArchiveReader};
    let mut reader = ProgramArchiveReader::from_reader(
        std::io::Cursor::new(bytes),
        KernelLoadOptions { validate: true },
    )
    .unwrap();
    let mut selected = reader
        .select(ProgramRecipe::ThresholdV1)
        .unwrap()
        .load_sector(0)
        .unwrap();
    let metadata = selected.threshold_metadata().unwrap();
    assert!(!metadata.full_original_scope().unwrap());
    assert!(metadata.require_global_proof().is_err());
    let semantic: serde_json::Value = serde_json::from_str(&metadata.semantic).unwrap();
    assert_eq!(semantic["projection_version"], 8);
    assert!(semantic.get("original_evidence").is_none());
    assert!(semantic.get("quotient_evidence").is_none());
    assert!(semantic["native_values"].is_string());
    let mut resident = 0;
    let mut nonresident = 0;
    for chart in &metadata.lineage().endpoint_charts {
        let m::MapDescriptor::ResolvedEndpointV1 {
            geometry, endpoint, ..
        } = &chart.map
        else {
            panic!("actual resolved map")
        };
        for n in endpoint.terms.iter().flat_map(|t| &t.numerators) {
            assert_eq!(n.source_factors, [2]);
            assert_eq!(n.degrees.len(), 2);
            assert_eq!(n.powers.len(), 2);
            if metadata
                .native_atom(&geometry.expressions, n.regular)
                .is_ok()
            {
                resident += 1;
            } else {
                nonresident += 1;
            }
        }
    }
    assert!(
        resident > 0 && nonresident > 0,
        "only the selected local expression table is resident"
    );
    let id = selected.content_id().to_owned();
    let mut values = Vec::new();
    for precision in [128, 256] {
        let mut value = vec![0.; selected.components().len()];
        selected.sectors_mut()[0]
            .replay_scaled(&[0.23, 0.37], &mut value, 1., precision)
            .unwrap();
        assert!(value.iter().all(|v| v.is_finite()));
        values.push(value);
    }
    let restored = KernelSet::from_bytes_with_options(
        &selected.to_bytes().unwrap(),
        KernelLoadOptions { validate: true },
    )
    .unwrap();
    assert_eq!(id, restored.content_id());
    serde_json::json!({"id": id, "values": values, "resident_numerators":resident, "nonresident_numerators":nonresident})
}

#[test]
fn genuine_v8_detaches_geometry_preserves_semantics_and_restores_local_numerators() {
    use crate::kernel::ProgramRecipe;
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let plan = paired_plan(first.path(), false);
    let renamed = paired_plan(second.path(), true);
    // The setup record exposes semantic identity without compiling each chart
    // twice. Source evidence must differ, while arithmetic stays the same.
    let before = plan.job(0).unwrap().run().unwrap();
    let after = renamed.job(0).unwrap().run().unwrap();
    let before = before.kernels().threshold_metadata().unwrap();
    let after = after.kernels().threshold_metadata().unwrap();
    assert_ne!(
        before.lineage().cells[0].native.raw_evidence,
        after.lineage().cells[0].native.raw_evidence
    );
    assert_eq!(before.semantic, after.semantic);
    drop(renamed);
    let mut writer = plan
        .archive_writer(std::io::Cursor::new(Vec::new()))
        .unwrap();
    for index in (0..plan.job_count()).rev() {
        let completion = plan.job(index).unwrap().run().unwrap();
        plan.validate_completion(&completion).unwrap();
        let mut bytes = Vec::new();
        let receipt = completion.write_record(&mut bytes).unwrap();
        writer
            .append_record(ProgramRecipe::ThresholdV1, &mut bytes.as_slice(), receipt)
            .unwrap();
    }
    let (cursor, _) = writer.finish().unwrap();
    let bytes = cursor.into_inner();
    let observation = selected_observation(bytes.clone());
    std::fs::write(first.path().join("paired.bin"), bytes).unwrap();
    std::fs::write(
        first.path().join("expected.json"),
        serde_json::to_vec(&observation).unwrap(),
    )
    .unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "kernel::threshold_owner::positive_projective::tests::positive_restore_child",
            "--ignored",
            "--test-threads=1",
        ])
        .env("FASTSECDEC_POSITIVE_PROJECTIVE_ARTIFACT", first.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore = "fresh-process helper invoked by genuine_v8_detaches_geometry_preserves_semantics_and_restores_local_numerators"]
fn positive_restore_child() {
    for i in 0..23 {
        Symbol::parse(format!("positive_restore_child::unrelated_{i}"), "").unwrap();
    }
    let root = std::path::PathBuf::from(
        std::env::var_os("FASTSECDEC_POSITIVE_PROJECTIVE_ARTIFACT").unwrap(),
    );
    let expected: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("expected.json")).unwrap()).unwrap();
    assert_eq!(
        selected_observation(std::fs::read(root.join("paired.bin")).unwrap()),
        expected
    );
}

#[test]
fn reversed_native_order_preserves_named_source_numerator_and_root_metadata() {
    use crate::threshold::{
        gcad::PositiveProjectiveQuotient,
        regularization::projective_pair_family::{ResolvedProjectivePairFamily, tests as fixture},
    };
    use symbolica::{domains::float::Complex, evaluate::OptimizationSettings};
    let (x, y, eps) = symbol!("pair_test::x", "pair_test::y", "pair_test::eps");
    let coefficient = Atom::one() + Atom::num(2) * Atom::i();
    // Declaration order and the numerator stay fixed. Only the native lifting
    // order changes, so the source x becomes the second retained CAD axis.
    let original = fixture::fixture(
        Some((&coefficient * Atom::var(x), 1)),
        Atom::one() + Atom::var(eps),
    );
    let mut problem = original.original_result().problem.clone();
    problem.solver.order = vec!["x2".into(), "x1".into(), "x0".into()];
    let raw = Arc::new(symgcad::solver::solve(&problem).unwrap());
    let owner =
        Arc::new(PositiveProjectiveQuotient::verify(original.preparation().clone(), raw).unwrap());
    assert_eq!(owner.preparation().coordinates(), &[x, y]);
    let units = vec![symbol!("reversed_pair::u"), symbol!("reversed_pair::v")];
    let limits = fixture::limits();
    let proposals = fixture::fixtures::proposals(&owner, &units, &limits).unwrap();
    let family = ResolvedProjectivePairFamily::admit(
        owner,
        units.clone(),
        proposals,
        limits,
        Default::default(),
        |_| ControlFlow::Continue(()),
    )
    .unwrap();
    assert!(
        family
            .charts()
            .iter()
            .all(|c| c.coordinate_order() == [y, x])
    );
    let continued = family
        .continue_symbolically(
            &GenerationOptions {
                max_order: 0,
                ..Default::default()
            },
            |_| ControlFlow::Continue(()),
        )
        .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let compiled = KernelSet::compile_threshold_projective_pair_family(
        &continued,
        directory.path(),
        0,
        Default::default(),
        CompilationSettings {
            backend: EvaluatorBackend::Eager,
            ..Default::default()
        },
    )
    .unwrap();
    let restored = KernelSet::from_bytes_with_options(
        &compiled.to_bytes().unwrap(),
        KernelLoadOptions { validate: true },
    )
    .unwrap();
    let metadata = restored.threshold_metadata().unwrap();
    assert_eq!(
        family.charts().len(),
        metadata.lineage().endpoint_charts.len()
    );
    for cell in &metadata.lineage().cells {
        assert_eq!(
            cell.coordinate_order
                .iter()
                .map(|s| metadata.native_symbol(*s).unwrap())
                .collect::<Vec<_>>(),
            [y, x]
        );
    }
    let tolerance = Rational::from((1, 2)).pow(80);
    let mut rejects_swapped_numerator = false;
    let mut branch_checks = 0;
    for (chart, published) in family
        .charts()
        .iter()
        .zip(&metadata.lineage().endpoint_charts)
    {
        let m::MapDescriptor::ResolvedEndpointV1 {
            geometry,
            endpoint,
            sections,
        } = &published.map
        else {
            panic!("resolved source map")
        };
        // Images are in original preparation [x,y] order, not CAD [y,x] order.
        assert_eq!(
            geometry
                .images
                .iter()
                .map(|a| metadata
                    .native_atom(&geometry.expressions, *a)
                    .unwrap()
                    .clone())
                .collect::<Vec<_>>(),
            chart.images()
        );
        let numerator = &endpoint.terms[0].numerators[0];
        assert_eq!(
            numerator
                .source_coordinates
                .iter()
                .map(|s| metadata.native_symbol(*s).unwrap())
                .collect::<Vec<_>>(),
            [x, y]
        );
        assert_eq!(numerator.source_factors, [2]);
        let mut transported = metadata
            .native_atom(&geometry.expressions, numerator.regular)
            .unwrap()
            .clone();
        for (unit, power) in units.iter().zip(&numerator.powers) {
            transported *= Atom::var(*unit).pow(*power);
        }
        let expected = &coefficient * &chart.images()[0];
        let deliberately_swapped = &coefficient * &chart.images()[1];
        let mut expressions = vec![transported - &expected, deliberately_swapped - expected];
        let branches = chart
            .sections()
            .iter()
            .flat_map(|s| s.branches())
            .collect::<Vec<_>>();
        assert_eq!(branches.len(), sections.len());
        // Branch images are retained in pre-half/pre-fan form. Transport this
        // independent reference through the native sector map and then bind
        // its root roles to the actual chart-local native RootProgram calls.
        let native_map = crate::generation::coordinates_from_parts(
            &units,
            fastsecdec_sectors::ParametricDomain::UnitCube,
            chart.sector_map(),
            &units,
        );
        let half = chart.endpoint_path()[1];
        let unit_replacements = units
            .iter()
            .enumerate()
            .map(|(axis, unit)| {
                let scaled = &native_map.images()[axis] / Atom::num(2);
                let image = if half & (1 << axis) == 0 {
                    scaled
                } else {
                    Atom::one() - scaled
                };
                Replacement::new(Pattern::Literal(Atom::var(*unit)), Pattern::Literal(image))
            })
            .collect::<Vec<_>>();
        let root_replacements = chart
            .sections()
            .iter()
            .zip(chart.root_inputs())
            .map(|(section, (tag, coefficients))| {
                assert_eq!(section.tag(), *tag);
                Replacement::new(
                    Pattern::Literal(Atom::var(section.root_symbol())),
                    Pattern::Literal(section.root_program().call(*tag, coefficients).unwrap()),
                )
            })
            .collect::<Vec<_>>();
        for (branch, section) in branches.into_iter().zip(sections) {
            let polynomial = metadata
                .native_atom(&geometry.expressions, section.polynomial)
                .unwrap();
            let role = metadata.native_symbol(section.root_variable).unwrap();
            let root_image = branch
                .image()
                .replace_multiple(&unit_replacements)
                .replace_multiple(&root_replacements);
            expressions.push(
                polynomial
                    .replace(Pattern::Literal(Atom::var(role)))
                    .with(Pattern::Literal(root_image)),
            );
            branch_checks += 1;
        }
        let program = Atom::evaluator_multiple(
            &expressions,
            &units.iter().copied().map(Atom::var).collect::<Vec<_>>(),
        )
        .optimization_settings(
            OptimizationSettings::default()
                .cores(1)
                .cpe_iterations(Some(0)),
        )
        .build()
        .unwrap();
        let bits = 128;
        let cf = |q: &Rational| {
            Complex::new(
                q.to_multi_prec_float(bits),
                Rational::zero().to_multi_prec_float(bits),
            )
        };
        let mut numeric = chart.roots().enter(bits, || {
            program.map_coeff_with_prec(
                &|q| {
                    Complex::new(
                        q.re.to_multi_prec_float(bits),
                        q.im.to_multi_prec_float(bits),
                    )
                },
                bits,
            )
        });
        let point = [Rational::from((1, 3)), Rational::from((2, 5))].map(|q| cf(&q));
        let mut values = vec![cf(&Rational::zero()); expressions.len()];
        numeric.try_evaluate(&point, &mut values).unwrap();
        for (index, value) in values.into_iter().enumerate() {
            let error = value
                .re
                .to_rational()
                .abs()
                .max(value.im.to_rational().abs());
            if index == 1 {
                rejects_swapped_numerator |= error > tolerance;
            } else {
                assert!(
                    error < tolerance,
                    "named-source numerator or original branch equation differs on chart {}",
                    published.id.0
                );
            }
        }
    }
    assert!(rejects_swapped_numerator && branch_checks > 0);
}

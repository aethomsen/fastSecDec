use super::*;
use crate::threshold::maps::endpoint_transport::{Error, FaceAxis, clearing::pull, tests::fixture};
use symbolica::{
    atom::AtomCore,
    domains::{atom::AtomField, rational::Rational},
    tensors::matrix::Matrix,
};

fn equal(a: &Atom, b: &Atom) {
    assert!((a - b).together().is_zero(), "{a} != {b}");
}
fn reconstruct(p: &super::super::ExtractedFactor, units: &[Symbol]) -> Atom {
    units
        .iter()
        .zip(p.orders())
        .map(|(s, e)| Atom::var(*s).pow(Atom::num(e.clone())))
        .product::<Atom>()
        * p.numerator()
        / p.denominator()
}
fn original() -> Arc<EndpointTransport> {
    let prefix = fixture();
    let faces = prefix
        .source()
        .unit_coordinates()
        .iter()
        .map(|&unit| FaceAxis {
            unit,
            side: FaceSide::Lower,
        })
        .collect::<Vec<_>>();
    EndpointTransport::prepare(prefix, &faces, &Limits::default(), || {
        ControlFlow::Continue(())
    })
    .unwrap()
}
fn determinant(images: &[Atom], units: &[Symbol]) -> Atom {
    Matrix::from_linear(
        images
            .iter()
            .flat_map(|p| units.iter().map(|s| p.derivative(*s)))
            .collect(),
        u32::try_from(units.len()).unwrap(),
        u32::try_from(units.len()).unwrap(),
        AtomField {
            statistical_zero_test: false,
            ..Default::default()
        },
    )
    .unwrap()
    .det()
    .unwrap()
}
#[test]
fn endpoint_child_cover_complete_replacement_and_original_term_reconstruction() {
    let transport = original();
    let axes = transport.target_units().to_vec();
    let cover = EndpointChartCover::prepare(
        transport.clone(),
        0,
        &axes,
        &[6],
        &Limits::default(),
        || ControlFlow::Continue(()),
    )
    .unwrap();
    assert!(cover.replacement_complete());
    assert!(!cover.closed_faces_certified());
    assert!(Arc::ptr_eq(cover.parent(), &transport));
    assert_eq!(cover.halves().len(), 8);
    let base = &transport.charts()[0];
    let targets = cover.target_units();
    assert!(
        targets
            .iter()
            .all(|s| !transport.target_units().contains(s))
    );
    let native = cover.halves()[6].geometry().unwrap();
    assert_eq!(cover.leaves().len(), 7 + native.sectors.len());
    let half_j = Atom::num((1, 8));
    for half in cover.halves() {
        assert_eq!(half.geometry().is_some(), half.mask() == 6);
        equal(
            &determinant(half.images(), targets),
            &(Atom::num(half.orientation()) * &half_j),
        );
        for (j, interval) in half.intervals().iter().enumerate() {
            assert_eq!(interval.axis(), j);
            assert_eq!(
                interval.half() == FaceSide::Upper,
                half.mask() & (1 << j) != 0
            );
            assert_eq!(
                interval.one_boundary(),
                ChildBoundary::Midpoint { unit: axes[j] }
            );
            assert_eq!(
                interval.zero_boundary(),
                ChildBoundary::Parent {
                    unit: axes[j],
                    side: interval.half()
                }
            );
            let inverse = if interval.half() == FaceSide::Lower {
                Atom::num(2) * &half.images()[j]
            } else {
                Atom::num(2) * (Atom::one() - &half.images()[j])
            };
            equal(&inverse, &Atom::var(targets[j]));
        }
    }
    let ancestor = transport.parent_prefix().source().unit_coordinates();
    let preparation = transport
        .parent_prefix()
        .source()
        .decomposition()
        .preparation();
    for (ordinal, leaf) in cover.leaves().iter().enumerate() {
        let chart = leaf.chart();
        assert_eq!(chart.ordinal(), ordinal);
        assert_eq!(chart.terms().len(), preparation.terms().len());
        let signed = determinant(leaf.parent_chart_images(), targets);
        assert!(
            (&signed - leaf.added_jacobian()).together().is_zero()
                || (&signed + leaf.added_jacobian()).together().is_zero()
        );
        equal(
            chart.extra_jacobian(),
            &(pull(
                base.extra_jacobian(),
                transport.target_units(),
                leaf.parent_chart_images(),
            ) * leaf.added_jacobian()),
        );
        for (got, p) in chart.unit_images().iter().zip(base.unit_images()) {
            equal(
                got,
                &pull(p, transport.target_units(), leaf.parent_chart_images()),
            );
        }
        for (got, p) in chart.source_images().iter().zip(base.source_images()) {
            equal(
                got,
                &pull(p, transport.target_units(), leaf.parent_chart_images()),
            );
        }
        for (got, p) in chart.root_equations().iter().zip(base.root_equations()) {
            equal(
                got,
                &pull(p, transport.target_units(), leaf.parent_chart_images()),
            );
        }
        for (i, factor) in chart.factors().iter().enumerate() {
            let source = &transport.identities()[i];
            equal(
                &reconstruct(factor, targets),
                &(pull(source.numerator(), ancestor, chart.unit_images())
                    / pull(source.denominator(), ancestor, chart.unit_images())),
            );
        }
        let measure = &transport.identities()[transport.measure_index()];
        equal(
            &reconstruct(chart.measure(), targets),
            &(pull(measure.numerator(), ancestor, chart.unit_images())
                / pull(measure.denominator(), ancestor, chart.unit_images())
                * chart.extra_jacobian()),
        );
        for (i, term) in preparation.terms().iter().enumerate() {
            // The fifth power clears both actual source exponents' rational
            // denominators. Check whole-term exact QQ values at two points;
            // the individual factor identities above are symbolic. No complex
            // principal-power distribution identity or root authority is used.
            let five = Atom::num(5);
            let expected = reconstruct(chart.measure(), targets).pow(5)
                * term
                    .monomial_powers()
                    .iter()
                    .zip(chart.source_images())
                    .map(|(e, p)| p.pow(e * &five))
                    .product::<Atom>()
                * term
                    .factors()
                    .iter()
                    .filter(|f| {
                        !f.exponent().is_zero()
                            && f.role() != crate::parametric::FactorRole::Polynomial
                    })
                    .map(|f| {
                        pull(
                            f.polynomial(),
                            preparation.coordinates(),
                            chart.source_images(),
                        )
                        .pow(f.exponent() * &five)
                    })
                    .product::<Atom>();
            let powers = chart.terms()[i].powers();
            assert_eq!(powers.len(), targets.len());
            assert_eq!(chart.terms()[i].source_index(), i);
            let mut actual = targets
                .iter()
                .zip(powers)
                .map(|(u, p)| Atom::var(*u).pow(p * &five))
                .product::<Atom>()
                * (chart.measure().numerator() / chart.measure().denominator()).pow(5);
            let links = &transport.term_links()[i];
            for (e, id) in term.monomial_powers().iter().zip(links.coordinates()) {
                if let Some(k) = id {
                    let f = &chart.factors()[*k];
                    actual *= (f.numerator() / f.denominator()).pow(e * &five);
                }
            }
            for (f, link) in term.factors().iter().zip(links.factors()) {
                if let super::super::FactorLink::Cleared(k) = link {
                    let p = &chart.factors()[*k];
                    actual *= (p.numerator() / p.denominator()).pow(f.exponent() * &five);
                }
            }
            let roles = targets
                .iter()
                .copied()
                .chain(
                    transport
                        .root_relations()
                        .iter()
                        .map(|r| r.section().root_symbol()),
                )
                .collect::<Vec<_>>();
            for point in 0..2 {
                let values = (0..roles.len())
                    .map(|j| Atom::num((j as i64 + 2 + point, j as i64 + 7 + point)))
                    .collect::<Vec<_>>();
                let actual = Rational::try_from(pull(&actual, &roles, &values).as_view()).unwrap();
                let expected =
                    Rational::try_from(pull(&expected, &roles, &values).as_view()).unwrap();
                assert_eq!(actual, expected);
            }
        }
        let view = cover.issued_chart(ordinal).unwrap();
        assert!(Arc::ptr_eq(view.transport(), &transport));
        assert!(Arc::ptr_eq(view.parent_prefix(), transport.parent_prefix()));
        for root in transport.root_relations() {
            let images = view.section_parameter_images(root.section()).unwrap();
            for (got, p) in images.iter().zip(&root.section().chart().parameter_images) {
                equal(got, &pull(p, ancestor, chart.unit_images()));
            }
        }
    }
    assert!(cover.issued_chart(cover.leaves().len()).is_err());
    assert!(transport.issued_chart(transport.charts().len()).is_err());
    assert!(transport.issued_chart(0).unwrap().cover().is_none());
}
#[test]
fn endpoint_child_cover_rejects_bad_inventory_and_preserves_retry() {
    let transport = original();
    let axes = transport.target_units();
    let limits = Limits::default();
    for (chart, split, masks) in [
        (usize::MAX, axes.to_vec(), vec![0]),
        (0, vec![axes[0], axes[0]], vec![0]),
        (
            0,
            vec![symbolica::symbol!("child_cover_test::foreign")],
            vec![0],
        ),
        (0, axes.to_vec(), vec![8]),
        (0, axes.to_vec(), vec![0, 0]),
    ] {
        assert!(matches!(
            EndpointChartCover::prepare(transport.clone(), chart, &split, &masks, &limits, || {
                ControlFlow::Continue(())
            }),
            Err(Error::Invalid(_))
        ));
    }
    assert!(matches!(
        EndpointChartCover::prepare(transport.clone(), 0, &[], &[], &limits, || {
            ControlFlow::Continue(())
        }),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        EndpointChartCover::prepare(
            transport.clone(),
            0,
            axes,
            &[],
            &Limits {
                max_entries: 0,
                ..limits.clone()
            },
            || ControlFlow::Continue(())
        ),
        Err(Error::ResourceIncomplete(_))
    ));
    assert!(matches!(
        EndpointChartCover::prepare(transport.clone(), 0, axes, &[0], &limits, || {
            ControlFlow::Break(())
        }),
        Err(Error::Cancelled)
    ));
    let full = EndpointChartCover::prepare(transport.clone(), 0, axes, &[], &limits, || {
        ControlFlow::Continue(())
    })
    .unwrap();
    assert_eq!(full.leaves().len(), 8);
    assert!(full.replacement_complete());
    let mut owned = Arc::try_unwrap(full).unwrap();
    owned.halves.pop();
    assert!(!owned.replacement_complete());
}

#[test]
fn endpoint_child_cover_subset_axes_and_genuine_foreign_section() {
    let transport = original();
    let axes = transport.target_units();
    let cover = EndpointChartCover::prepare(
        transport.clone(),
        0,
        &[axes[2], axes[0]],
        &[2],
        &Limits::default(),
        || ControlFlow::Continue(()),
    )
    .unwrap();
    assert_eq!(cover.split_axes(), [2, 0]);
    assert_eq!(cover.halves().len(), 4);
    assert!(cover.replacement_complete());
    for leaf in cover.leaves() {
        equal(
            &leaf.parent_chart_images()[1],
            &Atom::var(cover.target_units()[1]),
        );
        let signed = determinant(leaf.parent_chart_images(), cover.target_units());
        assert!(
            (&signed - leaf.added_jacobian()).together().is_zero()
                || (&signed + leaf.added_jacobian()).together().is_zero()
        );
    }
    let (power, proposal, _) = crate::threshold::maps::normalized::tests::fixture();
    let mut budget = crate::threshold::maps::regular::Budget::new(
        crate::threshold::maps::regular::Limits::default(),
    );
    let foreign = power.section(proposal, &mut budget).unwrap();
    let section = foreign
        .steps()
        .into_iter()
        .find_map(|step| match step {
            crate::threshold::maps::normalized::PrefixStep::Section { section, .. } => {
                Some(section.clone())
            }
            _ => None,
        })
        .unwrap();
    assert!(matches!(
        cover
            .issued_chart(0)
            .unwrap()
            .section_parameter_images(&section),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        transport
            .issued_chart(0)
            .unwrap()
            .section_parameter_images(&section),
        Err(Error::Invalid(_))
    ));
}

fn fifth_density(chart: &IssuedEndpointChart, index: usize) -> Atom {
    let transport = chart.transport();
    let c = chart.chart();
    let term = &transport
        .parent_prefix()
        .source()
        .decomposition()
        .preparation()
        .terms()[index];
    let mut value = chart
        .target_units()
        .iter()
        .zip(c.terms()[index].powers())
        .map(|(s, p)| Atom::var(*s).pow(p * Atom::num(5)))
        .product::<Atom>()
        * (c.measure().numerator() / c.measure().denominator()).pow(5);
    let links = &transport.term_links()[index];
    for (e, id) in term.monomial_powers().iter().zip(links.coordinates()) {
        if let Some(id) = id {
            let f = &c.factors()[*id];
            value *= (f.numerator() / f.denominator()).pow(e * Atom::num(5));
        }
    }
    for (f, link) in term.factors().iter().zip(links.factors()) {
        if let super::super::FactorLink::Cleared(id) = link {
            let f_res = &c.factors()[*id];
            value *= (f_res.numerator() / f_res.denominator()).pow(f.exponent() * Atom::num(5));
        }
    }
    value
}

#[test]
fn endpoint_recursive_cover_retains_all_ancestry_and_original_density() {
    let transport = original();
    let first = EndpointChartCover::prepare(
        transport.clone(),
        0,
        &[transport.target_units()[0]],
        &[],
        &Limits::default(),
        || ControlFlow::Continue(()),
    )
    .unwrap();
    assert_eq!(first.leaves().len(), 2);
    let selected = first.issued_chart(1).unwrap();
    let axes = selected.target_units().to_vec();
    let second = EndpointChartCover::refine(
        selected.clone(),
        &axes[1..],
        &[0],
        &Limits::default(),
        || ControlFlow::Continue(()),
    )
    .unwrap();
    assert!(first.replacement_complete() && second.replacement_complete());
    assert!(Arc::ptr_eq(second.parent(), &transport));
    assert!(Arc::ptr_eq(second.parent_chart().cover().unwrap(), &first));
    assert_eq!(second.parent_chart_index(), 1);
    // Only leaf1 is replaced. Leaf0 stays explicitly retained by the first cover.
    assert_eq!(first.leaves().len(), 2);
    assert_eq!(second.halves().len(), 4);
    assert_eq!(
        second.leaves().len(),
        3 + second.halves()[0].geometry().unwrap().sectors.len()
    );
    assert!(
        second
            .target_units()
            .iter()
            .all(|s| !first.target_units().contains(s) && !transport.target_units().contains(s))
    );
    for half in second.halves() {
        for interval in half.intervals() {
            assert!(first.target_units().contains(&interval.unit()));
            assert_eq!(
                interval.one_boundary(),
                ChildBoundary::Midpoint {
                    unit: interval.unit()
                }
            );
        }
    }
    for (ordinal, leaf) in second.leaves().iter().enumerate() {
        let view = second.issued_chart(ordinal).unwrap();
        assert!(Arc::ptr_eq(view.parent_prefix(), transport.parent_prefix()));
        let signed = determinant(leaf.parent_chart_images(), second.target_units());
        assert!(
            (&signed - leaf.added_jacobian()).together().is_zero()
                || (&signed + leaf.added_jacobian()).together().is_zero()
        );
        for (got, prior) in view
            .chart()
            .unit_images()
            .iter()
            .zip(selected.chart().unit_images())
        {
            equal(
                got,
                &pull(prior, selected.target_units(), leaf.parent_chart_images()),
            );
        }
        equal(
            view.chart().extra_jacobian(),
            &(pull(
                selected.chart().extra_jacobian(),
                selected.target_units(),
                leaf.parent_chart_images(),
            ) * leaf.added_jacobian()),
        );
        let ancestor = transport.parent_prefix().source().unit_coordinates();
        for root in transport.root_relations() {
            let got = view.section_parameter_images(root.section()).unwrap();
            for (got, prior) in got.iter().zip(&root.section().chart().parameter_images) {
                equal(got, &pull(prior, ancestor, view.chart().unit_images()));
            }
        }
        let roles = view
            .target_units()
            .iter()
            .copied()
            .chain(
                transport
                    .root_relations()
                    .iter()
                    .map(|r| r.section().root_symbol()),
            )
            .collect::<Vec<_>>();
        for term in 0..view.chart().terms().len() {
            let expected = pull(
                &fifth_density(&selected, term),
                selected.target_units(),
                leaf.parent_chart_images(),
            ) * leaf.added_jacobian().pow(5);
            let actual = fifth_density(&view, term);
            for offset in 0..2 {
                let point = (0..roles.len())
                    .map(|j| Atom::num((j as i64 + 2 + offset, j as i64 + 9 + offset)))
                    .collect::<Vec<_>>();
                assert_eq!(
                    Rational::try_from(pull(&actual, &roles, &point).as_view()).unwrap(),
                    Rational::try_from(pull(&expected, &roles, &point).as_view()).unwrap()
                );
            }
        }
    }
    assert!(matches!(
        EndpointChartCover::refine(
            selected,
            &[transport.target_units()[0]],
            &[],
            &Limits::default(),
            || ControlFlow::Continue(())
        ),
        Err(Error::Invalid(_))
    ));
}

//! Actual original coefficient owner for surviving nested-support controls.
use super::super::{
    coefficient::controlled_supported, embedding::SupportEmbedding, support::transport_embedding,
};
use super::*;
fn factor_order(
    h: Arc<ResolutionHistory>,
    source: Arc<MarkedIdeal>,
    ns: &str,
    b: &mut Budget,
) -> Arc<ComponentResidualOrder> {
    let mut f = ComponentFactorFrontier::new(h, source, ns.into(), b).unwrap();
    for _ in 0..100 {
        let Some(path) = f.pending().next().cloned() else {
            break;
        };
        assert!(matches!(
            f.advance(&path, &ComponentFactorLimits::default(), b)
                .unwrap(),
            FactorAdvance::Progress { .. }
        ));
    }
    let ComponentFactorCompletion::Complete(done) = f.try_complete().unwrap() else {
        panic!()
    };
    let leaf = done.nodes().values().find_map(|n| n.leaf()).unwrap();
    let ComponentResidualProduction::Order(o) =
        produce_component_residual_order(leaf.clone(), b).unwrap()
    else {
        panic!()
    };
    Arc::new(*o)
}
fn original(b: &mut Budget) -> Arc<CompanionCenter> {
    let r = Arc::new(
        Ring::new(
            vec![
                symbol!("original_nested::x"),
                symbol!("original_nested::y"),
                symbol!("original_nested::z"),
            ],
            vec![],
        )
        .unwrap(),
    );
    let local = LocalizedAlgebra::new(
        Ideal::new(r.clone(), vec![], b).unwrap(),
        vec![0, 1, 2],
        vec![],
        b,
    )
    .unwrap();
    let frame = Arc::new(
        EtaleCertificate {
            source: local,
            equations: vec![],
            dependent_axes: vec![],
            free_axes: vec![0, 1, 2],
            determinant_inverse_axis: None,
        }
        .verify(b)
        .unwrap(),
    );
    let SncProduction::Verified(ledger) = verify_initial_relative_snc(frame, vec![], b).unwrap()
    else {
        panic!()
    };
    let history = ResolutionHistory::initial(ledger).unwrap();
    let [x, y, z] = [
        r.coordinate(0).unwrap(),
        r.coordinate(1).unwrap(),
        r.coordinate(2).unwrap(),
    ];
    let source = Arc::new(
        MarkedIdeal::new(
            Ideal::new(r, vec![z.pow(2) + x.pow(3) + y.pow(3)], b).unwrap(),
            1,
            b,
        )
        .unwrap(),
    );
    let order = factor_order(history, source, "original_nested_factors", b);
    for i in 0..order.upper_order_cover().opens().len() {
        let CompanionOpenProduction::Contact(o) =
            produce_companion_open(order.clone(), i, &format!("original_nested_open{i}"), b)
                .unwrap()
        else {
            continue;
        };
        let o = Arc::new(*o);
        for j in 0..o.contact().candidates().len() {
            let q = construct_companion_coefficient(
                o.clone(),
                j,
                [
                    symbol!(format!("original_nested_q{i}_{j}")),
                    symbol!(format!("original_nested_i{i}_{j}")),
                ],
                b,
            )
            .unwrap();
            let CompanionCoefficientProduction::Coefficient(q) = q else {
                continue;
            };
            let mut driver =
                CompanionFirstCenter::new(Arc::new(*q), "original_nested_first", false).unwrap();
            for _ in 0..100 {
                match driver.advance(b).unwrap() {
                    RecursiveAdvance::Complete => break,
                    RecursiveAdvance::Progress => {}
                    RecursiveAdvance::Incomplete { reason } => {
                        panic!("original first incomplete {reason}")
                    }
                }
            }
            if let CompanionCenterOutcome::Center(c) = driver.outcome() {
                return c.clone();
            }
        }
    }
    panic!("no original source center")
}
#[test]
fn original_native_surviving_child_anchor_and_same_physical_transform() {
    let mut b = budget();
    let outer = original(&mut b);
    let origin = EmbeddedChildCycle::new(outer.clone(), &mut b).unwrap();
    let problem = origin.initial_problem().unwrap();
    let mut recursion = problem
        .recursion("original_nested_problem".into(), &mut b)
        .unwrap();
    for _ in 0..1000 {
        match recursion
            .advance(&ComponentFactorLimits::default(), &mut b)
            .unwrap()
        {
            RecursiveAdvance::Complete => break,
            RecursiveAdvance::Progress => {}
            RecursiveAdvance::Incomplete { reason } => {
                panic!("original problem incomplete {reason}")
            }
        }
    }
    let ready = recursion
        .records()
        .iter()
        .find_map(|r| {
            if let ProblemRecursionRecord::Center(c) = r {
                Some(c.clone())
            } else {
                None
            }
        })
        .unwrap();
    let EmbeddedProblemProduction::OriginalCenter(physical) = ready.ascend(&mut b).unwrap() else {
        panic!("original ascent incomplete")
    };
    let anchored =
        AnchoredOriginalTree::new(OriginalAnchorCenter::Original(physical.clone()), &mut b)
            .unwrap();
    let child = EmbeddedChildCycle::new(ready.center().clone(), &mut b).unwrap();
    let base = SupportEmbedding::contact(outer.coefficient().contact().clone(), &mut b).unwrap();
    assert!(Arc::ptr_eq(
        base.frame(),
        problem.history().ledger().frame()
    ));
    let mut embed = base
        .restrict_unit_open(
            ready.center().coefficient().source().restriction().clone(),
            &mut b,
        )
        .unwrap();
    let (ideal, _, _) = super::super::super::recursive::lift_embedded_geometry(
        outer.frame(),
        outer.parent_source(),
        ready.center().frame(),
        embed.extension(),
        embed.equations().generators().to_vec(),
        ready.center().ideal(),
        ready.center().normals(),
        &mut b,
    )
    .unwrap();
    let local = outer.frame().local();
    assert!(equal(
        &local.ideal().sum(&ideal, &mut b).unwrap(),
        &local.ideal().sum(outer.ideal(), &mut b).unwrap(),
        local.unit_relations(),
        &mut b
    ));
    let problem_embedding = embed.clone();
    let mut embeds = vec![];
    let mut contact = Some(ready.center().coefficient().contact().clone());
    for level in child.original_tree().levels() {
        embed = SupportEmbedding::nested(embed, contact.take().unwrap(), &mut b).unwrap();
        let local = embed.frame().local();
        let restricted = embed.extension().ideal(&ideal, &mut b).unwrap();
        assert!(equal(
            &local.ideal().sum(&restricted, &mut b).unwrap(),
            &local.ideal().sum(level.center().ideal(), &mut b).unwrap(),
            local.unit_relations(),
            &mut b
        ));
        assert!(Arc::ptr_eq(embed.frame(), level.history().ledger().frame()));
        contact = level
            .center()
            .lift_receipt()
            .map(|r| r.level().contact().clone());
        embeds.push(embed.clone());
    }
    let checked =
        CheckedRecursiveCenter::new(RecursiveCenterOrigin::OriginalProblem(physical), &mut b)
            .unwrap();
    let adapt = adapt_recursive_center(checked, "original_nested_adapt", &mut b).unwrap();
    let blowup = blowup_recursive_center(adapt, "original_nested_blowup", &mut b).unwrap();
    let mut nonempty = 0;
    let mut empty = 0;
    let mut source_survives = 0;
    let mut source_empty = 0;
    let mut g_empty = 0;
    for (pi, chart) in blowup.charts().iter().enumerate() {
        let carried = SupportedProblemChart::prepare(
            anchored.clone(),
            chart.clone(),
            &format!("original_current{pi}"),
            &mut b,
        )
        .unwrap();
        let ancestors = carried
            .ancestor_presentations(&format!("original_ancestors{pi}"), &mut b)
            .unwrap();
        println!("ancestor full C/J presentations{}", ancestors.len());
        for (oi, current) in carried.opens().iter().enumerate() {
            carried.check_open(current).unwrap();
            let mut factors = ComponentFactorFrontier::new(
                current.history().clone(),
                current.source().target().clone(),
                format!("original_drop_factors{pi}_{oi}"),
                &mut b,
            )
            .unwrap();
            for _ in 0..100 {
                let Some(path) = factors.pending().next().cloned() else {
                    break;
                };
                assert!(matches!(
                    factors
                        .advance(&path, &ComponentFactorLimits::default(), &mut b)
                        .unwrap(),
                    FactorAdvance::Progress { .. }
                ));
            }
            let ComponentFactorCompletion::Complete(factors) = factors.try_complete().unwrap()
            else {
                panic!("drop factors incomplete")
            };
            let inventory = SupportedDropInventory::prove(
                carried.clone(),
                current.clone(),
                Arc::new(factors),
                &mut b,
            )
            .unwrap();
            println!(
                "actual drop pivot{pi} open{oi} max{:?} records{} source-empty{}",
                inventory.proved_maximum(),
                inventory.records().len(),
                current.source_cosupport().empty()
            );
            for (ri, record) in inventory.records().iter().enumerate() {
                if matches!(record, SupportedDropRecord::Positive(_)) {
                    let drop = inventory.positive_drop(ri).unwrap();
                    let cycle = CycleSnapshot::after_supported_drop(drop.clone(), &mut b).unwrap();
                    println!(
                        "positive drop current{} old IDs{}",
                        drop.current().algebraic_maximum_on_cosupport(),
                        cycle.old_ids().len()
                    );
                    for ni in 0..drop.current().upper_order_cover().opens().len() {
                        let open = produce_companion_open(
                            drop.current().clone(),
                            ni,
                            &format!("next_open{pi}_{oi}_{ri}_{ni}"),
                            &mut b,
                        )
                        .unwrap();
                        let CompanionOpenProduction::Contact(open) = open else {
                            println!("next companion not at maximum");
                            continue;
                        };
                        let open = Arc::new(*open);
                        for ci in 0..open.contact().candidates().len() {
                            let q = construct_companion_coefficient_for_cycle(
                                open.clone(),
                                ci,
                                [
                                    symbol!(format!("next_q{pi}_{oi}_{ri}_{ni}_{ci}")),
                                    symbol!(format!("next_inv{pi}_{oi}_{ri}_{ni}_{ci}")),
                                ],
                                cycle.clone(),
                                &mut b,
                            )
                            .unwrap();
                            let CompanionCoefficientProduction::Coefficient(q) = q else {
                                continue;
                            };
                            let mut driver = CompanionFirstCenter::new(
                                Arc::new(*q),
                                &format!("next_center{pi}_{oi}_{ri}_{ni}_{ci}"),
                                false,
                            )
                            .unwrap();
                            for _ in 0..100 {
                                match driver.advance(&mut b).unwrap() {
                                    RecursiveAdvance::Complete => break,
                                    RecursiveAdvance::Progress => {}
                                    RecursiveAdvance::Incomplete { reason } => {
                                        println!("next driver pending{reason}");
                                        break;
                                    }
                                }
                            }
                            if let CompanionCenterOutcome::Center(next) = driver.outcome() {
                                println!(
                                    "next actual center mark{} dimension{} normals{}",
                                    next.parent_source().mark(),
                                    next.frame().free_axes().len(),
                                    next.normals().len()
                                );
                                match ContinuedSupportedCenter::prepare(
                                    drop.clone(),
                                    next.clone(),
                                    &format!("next_ancestor{pi}_{oi}_{ri}_{ni}_{ci}"),
                                    &mut b,
                                )
                                .unwrap()
                                {
                                    ContinuedSupportedProduction::NeedsPhysicalLocalization {
                                        ..
                                    } => println!("next ambient localization pending"),
                                    ContinuedSupportedProduction::Center(next) => {
                                        println!(
                                            "next exact physical lift PASS ancestors{}",
                                            next.ancestors().len()
                                        );
                                        let checked = CheckedRecursiveCenter::new(
                                            RecursiveCenterOrigin::SupportedProblem(next.clone()),
                                            &mut b,
                                        )
                                        .unwrap();
                                        let adapted = adapt_recursive_center(
                                            checked,
                                            &format!("next_adapt{pi}_{oi}_{ri}_{ni}_{ci}"),
                                            &mut b,
                                        )
                                        .unwrap();
                                        let next_blowup = blowup_recursive_center(
                                            adapted,
                                            &format!("next_blowup{pi}_{oi}_{ri}_{ni}_{ci}"),
                                            &mut b,
                                        )
                                        .unwrap();
                                        assert!(matches!(
                                            ContinuedSupportedFrontier::new(
                                                Arc::new((*next).clone()),
                                                next_blowup.clone(),
                                                "wrong_center".into(),
                                                &mut b
                                            ),
                                            Err(Error::Invalid(_))
                                        ));
                                        let mut frontier = ContinuedSupportedFrontier::new(
                                            next.clone(),
                                            next_blowup.clone(),
                                            format!("next_carry{pi}_{oi}_{ri}_{ni}_{ci}"),
                                            &mut b,
                                        )
                                        .unwrap();
                                        let mut next_opens = 0;
                                        let mut next_ancestors = 0;
                                        let mut pending = 0;
                                        let mut exact_empty = 0;
                                        let mut rejected_frames = 0;
                                        let mut checked_nonempty = false;
                                        let before = frontier.states().len();
                                        for vi in 0..next_blowup.charts().len() {
                                            match frontier.advance(vi, &mut b).unwrap() {
                                                ContinuedChartAdvance::Incomplete { reason } => {
                                                    pending += 1;
                                                    println!(
                                                        "retained second chart{vi} pending {reason}"
                                                    );
                                                    if matches!(vi, 4 | 5 | 7) {
                                                        let ancestor = &next.ancestors()[0];
                                                        let embedding = SupportEmbedding::strict(
                                                            ancestor.support().clone(),
                                                            ancestor.open().clone(),
                                                            &mut b,
                                                        )
                                                        .unwrap();
                                                        let prep =
                                                            StrictSupportPreparation::prepare(
                                                                next_blowup.charts()[vi]
                                                                    .geometry()
                                                                    .clone(),
                                                                embedding,
                                                                &format!("nonunit_chart{vi}"),
                                                                &mut b,
                                                            )
                                                            .unwrap();
                                                        assert!(matches!(
                                                            prep.physical_cover(&mut b),
                                                            Err(Error::ResourceIncomplete(
                                                                "original guard is nonunit on strict support"
                                                            ))
                                                        ));
                                                    }
                                                    if vi == 8 {
                                                        let chart =
                                                            next_blowup.charts()[vi].clone();
                                                        let anchor = AnchoredOriginalTree::new(
                                                            OriginalAnchorCenter::Continued(
                                                                next.clone(),
                                                            ),
                                                            &mut b,
                                                        )
                                                        .unwrap();
                                                        let prep =
                                                            StrictSupportPreparation::prepare(
                                                                chart.geometry().clone(),
                                                                anchor.embeddings()[0].clone(),
                                                                "actual_chart8_prepare",
                                                                &mut b,
                                                            )
                                                            .unwrap();
                                                        let owner =
                                                            StrictSupportRefinement::prepare(
                                                                chart.clone(),
                                                                prep,
                                                                &mut b,
                                                            )
                                                            .unwrap();
                                                        assert_eq!(owner.cover().opens().len(), 4);
                                                        assert!(matches!(
                                                            ContinuedRefinementFrontier::new(
                                                                Arc::new((*next).clone()),
                                                                owner.clone(),
                                                                "wrong_refinement_center".into(),
                                                                &mut b
                                                            ),
                                                            Err(Error::Invalid(_))
                                                        ));
                                                        let mut refined =
                                                            ContinuedRefinementFrontier::new(
                                                                next.clone(),
                                                                owner,
                                                                "actual_chart8".into(),
                                                                &mut b,
                                                            )
                                                            .unwrap();
                                                        let mut stopped = Budget::new(Limits {
                                                            max_operations: 0,
                                                            ..Limits::default()
                                                        });
                                                        assert!(matches!(
                                                            refined
                                                                .advance(0, &mut stopped)
                                                                .unwrap(),
                                                            ContinuedChartAdvance::Incomplete { .. }
                                                        ));
                                                        assert!(!refined.complete_inventory());
                                                        for side in
                                                            (0..refined.states().len()).rev()
                                                        {
                                                            assert_eq!(
                                                                refined
                                                                    .advance(side, &mut b)
                                                                    .unwrap(),
                                                                ContinuedChartAdvance::Complete
                                                            );
                                                        }
                                                        assert!(refined.complete_inventory());
                                                        let mut accepted = 0;
                                                        let mut truly_empty = 0;
                                                        for (side, state) in
                                                            refined.states().iter().enumerate()
                                                        {
                                                            match state{
                                                            ContinuedRefinementState::Carried{localization,chart:done}=>{
                                                                accepted+=1;assert_eq!(done.ancestors().len(),next.ancestors().len());
                                                                assert!(done.chart().history().same_root(chart.history()));assert_eq!(done.chart().history().births(),chart.history().births());assert_eq!(done.chart().history().stage(),chart.history().stage());
                                                                assert!(Arc::ptr_eq(done.chart().geometry().target_restriction().unwrap().1,localization));
                                                                println!("ACTUAL_CHART8 side{side} full hierarchy PASS ancestors{} child-levels{} opens{}",done.ancestors().len(),done.child().levels().len(),done.opens().len());
                                                            },
                                                            ContinuedRefinementState::EmptyPrincipal{..}=>truly_empty+=1,
                                                            _=>panic!("pending physical open"),
                                                        }
                                                        }
                                                        assert!(accepted > 0);
                                                        assert_eq!(accepted + truly_empty, 4);
                                                        let mut stopped = Budget::new(Limits {
                                                            max_operations: 0,
                                                            ..Limits::default()
                                                        });
                                                        for side in 0..refined.states().len() {
                                                            assert_eq!(refined.advance(side,&mut stopped).unwrap(),ContinuedChartAdvance::AlreadyComplete);
                                                        }
                                                        assert_eq!(stopped.operations(), 0);
                                                        println!(
                                                            "ACTUAL_CHART8 full physical cover retained {accepted} carried, {truly_empty} genuinely empty; births unchanged"
                                                        );
                                                    }
                                                }
                                                ContinuedChartAdvance::Complete => {
                                                    let ContinuedChartState::Complete(transition) =
                                                        &frontier.states()[vi]
                                                    else {
                                                        panic!()
                                                    };
                                                    assert!(Arc::ptr_eq(
                                                        transition.chart(),
                                                        &next_blowup.charts()[vi]
                                                    ));
                                                    next_opens += transition.opens().len();
                                                    next_ancestors += transition
                                                        .ancestors()
                                                        .iter()
                                                        .map(|t| t.opens().len())
                                                        .sum::<usize>();
                                                    let supports = transition
                                                        .ancestors()
                                                        .iter()
                                                        .map(|a| a.support())
                                                        .chain(
                                                            transition
                                                                .child()
                                                                .levels()
                                                                .iter()
                                                                .map(|l| l.support()),
                                                        )
                                                        .chain(std::iter::once(
                                                            transition.support(),
                                                        ));
                                                    let mut chart_empty = 0;
                                                    for support in supports {
                                                        let sat = support.saturation();
                                                        let local = sat.local();
                                                        assert!(
                                                            !local
                                                                .zero(&local.ring().one(), &mut b)
                                                                .unwrap()
                                                        );
                                                        if sat.empty() {
                                                            assert!(
                                                                sat.result()
                                                                    .contains(
                                                                        &local.ring().one(),
                                                                        local.unit_relations(),
                                                                        &mut b
                                                                    )
                                                                    .unwrap()
                                                            );
                                                            assert!(support.opens().is_empty());
                                                            chart_empty += 1;
                                                            exact_empty += 1;
                                                        } else if !checked_nonempty {
                                                            let forged = Ideal::new(
                                                                local.ring().clone(),
                                                                vec![local.ring().one()],
                                                                &mut b,
                                                            )
                                                            .unwrap();
                                                            assert!(
                                                                !sat.verify_candidate(
                                                                    &forged,
                                                                    "reject_false_empty",
                                                                    &mut b
                                                                )
                                                                .unwrap()
                                                            );
                                                            assert!(
                                                                OpenCoverCertificate {
                                                                    algebra: local.clone(),
                                                                    support: (**sat.result())
                                                                        .clone(),
                                                                    opens: vec![]
                                                                }
                                                                .verify(&mut b)
                                                                .is_err()
                                                            );
                                                            checked_nonempty = true;
                                                        }
                                                        for rejected in support.unresolved_frames()
                                                        {
                                                            assert!(
                                                                !rejected
                                                                    .selected()
                                                                    .zero(
                                                                        rejected.missing_relation(),
                                                                        &mut b
                                                                    )
                                                                    .unwrap()
                                                            );
                                                            assert!(
                                                                rejected
                                                                    .relations()
                                                                    .contains(
                                                                        rejected.missing_relation(),
                                                                        rejected
                                                                            .selected()
                                                                            .unit_relations(),
                                                                        &mut b
                                                                    )
                                                                    .unwrap()
                                                            );
                                                            rejected_frames += 1;
                                                        }
                                                    }
                                                    if vi == 3 {
                                                        assert!(
                                                            chart_empty > 0,
                                                            "fourth issued chart retains exact auxiliary emptiness"
                                                        );
                                                    }
                                                    let ops = b.operations();
                                                    assert_eq!(
                                                        frontier.advance(vi, &mut b).unwrap(),
                                                        ContinuedChartAdvance::AlreadyComplete
                                                    );
                                                    assert_eq!(ops, b.operations());
                                                }
                                                ContinuedChartAdvance::AlreadyComplete => panic!(),
                                            }
                                        }
                                        assert_eq!(frontier.states().len(), before);
                                        assert_eq!(
                                            frontier.states().len(),
                                            next_blowup.charts().len()
                                        );
                                        assert!(exact_empty > 0);
                                        assert!(rejected_frames > 0);
                                        assert!(checked_nonempty);
                                        if let Some(index) =
                                            frontier.states().iter().position(|s| {
                                                matches!(s, ContinuedChartState::Pending { .. })
                                            })
                                        {
                                            let mut none = Budget::new(Limits {
                                                max_operations: 0,
                                                ..Limits::default()
                                            });
                                            assert!(matches!(
                                                frontier.advance(index, &mut none).unwrap(),
                                                ContinuedChartAdvance::Incomplete { .. }
                                            ));
                                            assert!(!frontier.complete_inventory());
                                            assert!(matches!(
                                                frontier.clone().try_complete(),
                                                ContinuedFrontierCompletion::Incomplete(_)
                                            ));
                                        }
                                        assert_eq!(frontier.complete_inventory(), pending == 0);
                                        assert!(matches!(
                                            frontier.advance(usize::MAX, &mut b),
                                            Err(Error::Invalid(_))
                                        ));
                                        println!(
                                            "actual second physical transform pivots{} supported opens{next_opens} full ancestor C/J opens{next_ancestors} pending{pending}",
                                            next_blowup.charts().len()
                                        );
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }
        let checked_tree = anchored
            .pull_first(chart.clone(), &format!("original_tree{pi}"), &mut b)
            .unwrap();
        let _ = checked_tree.histories(&mut b).unwrap();
        for (li, (level, embed)) in child
            .original_tree()
            .levels()
            .iter()
            .zip(&embeds)
            .enumerate()
        {
            let support = transport_embedding(
                chart.geometry().clone(),
                embed.clone(),
                &format!("original_nested_support{pi}_{li}"),
                &mut b,
            )
            .unwrap();
            empty += usize::from(support.saturation().empty());
            let center_source =
                Arc::new(MarkedIdeal::new(level.center().ideal().clone(), 1, &b).unwrap());
            for (oi, open) in support.opens().iter().enumerate() {
                let source = controlled_supported(
                    support.clone(),
                    open.clone(),
                    level.center().source().clone(),
                    &format!("original_nested_source{pi}_{li}_{oi}"),
                    &mut b,
                )
                .unwrap();
                let center = controlled_supported(
                    support.clone(),
                    open.clone(),
                    center_source.clone(),
                    &format!("original_nested_center{pi}_{li}_{oi}"),
                    &mut b,
                )
                .unwrap();
                let history = level
                    .history()
                    .advance_first_embedded(level.center(), chart, &source, &center, &mut b)
                    .unwrap();
                assert!(history.same_root(level.history()));
                assert_eq!(
                    history.stage(),
                    u64::from(chart.geometry().exceptional().is_some())
                );
                nonempty += 1;
                println!(
                    "new level{li} pivot{pi} open{oi} history{} source mark{} target{}",
                    history.stage(),
                    source.target().mark(),
                    source
                        .target()
                        .ideal()
                        .generators()
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(";")
                );
            }
        }
        // Current original marked J and its newly constructed supported G are
        // carried separately from the outer ambient G. These native diagnostic
        // proofs do not mint a fresh cycle from a transformed J.
        let support = transport_embedding(
            chart.geometry().clone(),
            problem_embedding.clone(),
            &format!("original_nested_problem_support{pi}"),
            &mut b,
        )
        .unwrap();
        let g_source = Arc::new(
            MarkedIdeal::new(
                (**ready.center().coefficient().contact().source().source()).clone(),
                ready
                    .center()
                    .coefficient()
                    .contact()
                    .source()
                    .algebraic_maximum_order(),
                &b,
            )
            .unwrap(),
        );
        for (oi, open) in support.opens().iter().enumerate() {
            let source = controlled_supported(
                support.clone(),
                open.clone(),
                ready.center().parent_source().clone(),
                &format!("original_nested_current{pi}_{oi}"),
                &mut b,
            )
            .unwrap();
            let g = controlled_supported(
                support.clone(),
                open.clone(),
                g_source.clone(),
                &format!("original_nested_G{pi}_{oi}"),
                &mut b,
            )
            .unwrap();
            let source_proof =
                MarkedCosupport::prove(open.frame().clone(), source.target().clone(), &mut b)
                    .unwrap();
            let g_proof =
                MarkedCosupport::prove(open.frame().clone(), g.target().clone(), &mut b).unwrap();
            source_empty += usize::from(source_proof.empty());
            source_survives += usize::from(!source_proof.empty());
            g_empty += usize::from(g_proof.empty());
        }
    }
    assert!(nonempty > 0 && empty > 0);
    println!(
        "genuine original nested anchor: levels{} pivots{} surviving support opens{nonempty} empty supports{empty} marked J survives{source_survives} empty{source_empty} supported G empty{g_empty}",
        embeds.len(),
        blowup.charts().len()
    );
}

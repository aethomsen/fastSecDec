use super::*;
fn complete(
    history: Arc<ResolutionHistory>,
    source: Arc<MarkedIdeal>,
    ns: &str,
    b: &mut Budget,
) -> Arc<CompletedComponentFactors> {
    let mut f = ComponentFactorFrontier::new(history, source, ns.into(), b).unwrap();
    for _ in 0..100 {
        let Some(p) = f.pending().next().cloned() else {
            break;
        };
        f.advance(&p, &ComponentFactorLimits::default(), b).unwrap();
    }
    let ComponentFactorCompletion::Complete(f) = f.try_complete().unwrap() else {
        panic!()
    };
    Arc::new(f)
}
fn center(
    order: Arc<ComponentResidualOrder>,
    cycle: Option<Arc<CycleSnapshot>>,
    ns: &str,
    b: &mut Budget,
) -> Arc<CompanionCenter> {
    for oi in 0..order.upper_order_cover().opens().len() {
        let CompanionOpenProduction::Contact(o) =
            produce_companion_open(order.clone(), oi, &format!("{ns}_o{oi}"), b).unwrap()
        else {
            continue;
        };
        let o = Arc::new(*o);
        for ci in 0..o.contact().candidates().len() {
            let symbols = [
                symbol!(format!("{ns}_q{oi}_{ci}")),
                symbol!(format!("{ns}_i{oi}_{ci}")),
            ];
            let q = match &cycle {
                Some(c) => {
                    construct_companion_coefficient_for_cycle(o.clone(), ci, symbols, c.clone(), b)
                }
                None => construct_companion_coefficient(o.clone(), ci, symbols, b),
            }
            .unwrap();
            let CompanionCoefficientProduction::Coefficient(q) = q else {
                continue;
            };
            let mut d = CompanionFirstCenter::new(Arc::new(*q), &format!("{ns}_r{oi}_{ci}"), false)
                .unwrap();
            for _ in 0..100 {
                let step = d.advance(b).unwrap();
                if !matches!(step, RecursiveAdvance::Progress) {
                    println!("PLANE step {ns} {oi}/{ci} {step:?}");
                    break;
                }
            }
            if let CompanionCenterOutcome::Center(c) = d.outcome() {
                return c.clone();
            }
            println!(
                "PLANE outcome {}",
                match d.outcome() {
                    CompanionCenterOutcome::Pending => "pending",
                    CompanionCenterOutcome::ChildComponentOpensRequired(_) => "component",
                    CompanionCenterOutcome::ChildNeedsMoreOpens(_) => "contactcover",
                    CompanionCenterOutcome::EmptyChild(_) => "empty",
                    _ => "center",
                }
            );
        }
    }
    panic!("no plane center")
}
fn maps(c: Arc<CompanionCenter>, ns: &str, b: &mut Budget) -> Arc<RelativeRecursiveBlowup> {
    let checked = CheckedRecursiveCenter::new(RecursiveCenterOrigin::Companion(c), b).unwrap();
    let adapted = adapt_recursive_center(checked, &format!("{ns}_adapt"), b).unwrap();
    blowup_recursive_center(adapted, &format!("{ns}_maps"), b).unwrap()
}
fn initial_order(px: usize, py: usize, b: &mut Budget) -> Arc<ComponentResidualOrder> {
    let ring = Arc::new(
        Ring::new(
            vec![symbol!("physical_plane::x"), symbol!("physical_plane::y")],
            vec![],
        )
        .unwrap(),
    );
    let local = LocalizedAlgebra::new(
        Ideal::new(ring.clone(), vec![], b).unwrap(),
        vec![0, 1],
        vec![],
        b,
    )
    .unwrap();
    let frame = Arc::new(
        EtaleCertificate {
            source: local,
            equations: vec![],
            dependent_axes: vec![],
            free_axes: vec![0, 1],
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
    let f = ring.coordinate(0).unwrap().pow(px) + ring.coordinate(1).unwrap().pow(py);
    let source = Arc::new(MarkedIdeal::new(Ideal::new(ring, vec![f], b).unwrap(), 1, b).unwrap());
    let factors = complete(history, source, "plane_initial", b);
    let leaf = factors.nodes().values().find_map(|n| n.leaf()).unwrap();
    let ComponentResidualProduction::Order(order) =
        produce_component_residual_order(leaf.clone(), b).unwrap()
    else {
        panic!()
    };
    Arc::new(*order)
}
#[test]
fn initial_problem_positive_drop_keeps_original_root_and_history() {
    let mut b = budget();
    let order = initial_order(2, 3, &mut b);
    let c = center(order, None, "plane_initial", &mut b);
    let problem = OriginalPhysicalProblem::new(c.clone(), &mut b).unwrap();
    let first = maps(c, "plane_initial", &mut b);
    let mut stack = Vec::new();
    for (i, c) in first.charts().iter().enumerate() {
        stack.push((format!("{i}"), problem.advance(c.clone(), &mut b).unwrap()));
    }
    let mut resets = 0;
    let mut terminals = 0;
    while let Some((path, state)) = stack.pop() {
        assert!(Arc::ptr_eq(state.root(), problem.root()));
        let factors = complete(
            state.chart().history().clone(),
            state.chart().parent().target().clone(),
            &format!("plane_f{path}"),
            &mut b,
        );
        let terminal = PhysicalTerminalInventory::prove(
            state.clone(),
            factors.clone(),
            &format!("plane_t{path}"),
            &mut b,
        )
        .unwrap();
        if terminal.all_local_factors_snc() {
            terminals += 1;
            continue;
        }
        println!("PLANE pending{path} depth{}", state.depth());
        let drop =
            PhysicalDropInventory::prove(state.clone(), factors, &format!("plane_G{path}"), &mut b)
                .unwrap();
        println!("PLANE maximum{:?}", drop.proved_maximum());
        let ri = drop
            .records()
            .iter()
            .position(|r| matches!(r, AmbientDropRecord::Positive(_)))
            .unwrap();
        let drop = drop.positive_drop(ri).unwrap();
        let cycle = CycleSnapshot::after_physical_drop(drop.clone(), &mut b).unwrap();
        assert!(Arc::ptr_eq(cycle.birth_history(), state.chart().history()));
        assert_eq!(
            cycle.birth_history().births(),
            state.chart().history().births()
        );
        let c = center(
            drop.current().clone(),
            Some(cycle),
            &format!("plane_next{path}"),
            &mut b,
        );
        assert!(matches!(
            PhysicalContinuationCenter::prepare(
                drop.inventory().positive_drop(ri).unwrap(),
                c.clone(),
                &mut b
            ),
            Err(Error::Invalid(_))
        ));
        let owner = PhysicalContinuationCenter::prepare(drop, c.clone(), &mut b).unwrap();
        let start = state.begin_cycle(owner, &mut b).unwrap();
        let next = maps(c, &format!("plane_next{path}"), &mut b);
        assert!(matches!(
            PhysicalChartFrontier::from_start(
                start.clone(),
                first.clone(),
                "plane_wrong_first".into(),
                &mut b
            ),
            Err(Error::Invalid(_))
        ));
        resets += 1;
        assert!(resets < 10);
        for (i, c) in next.charts().iter().enumerate() {
            stack.push((
                format!("{path}_{i}"),
                start
                    .advance(c.clone(), &format!("plane_state{path}_{i}"), &mut b)
                    .unwrap(),
            ));
        }
    }
    assert_eq!(resets, 1);
    assert_eq!(terminals, 7);
    println!("PLANE completed resets{resets} terminals{terminals}");
}

#[test]
fn higher_cusp_retains_coefficient_mark_resource_frontier() {
    let mut b = budget();
    assert_eq!(b.limits.max_mark, 64);
    let order = initial_order(3, 5, &mut b);
    for oi in 0..order.upper_order_cover().opens().len() {
        let CompanionOpenProduction::Contact(open) =
            produce_companion_open(order.clone(), oi, &format!("limited_cusp_o{oi}"), &mut b)
                .unwrap()
        else {
            continue;
        };
        let open = Arc::new(*open);
        for ci in 0..open.contact().candidates().len() {
            let CompanionCoefficientProduction::Coefficient(q) = construct_companion_coefficient(
                open.clone(),
                ci,
                [
                    symbol!(format!("limited_cusp_q{oi}_{ci}")),
                    symbol!(format!("limited_cusp_i{oi}_{ci}")),
                ],
                &mut b,
            )
            .unwrap() else {
                continue;
            };
            let q = Arc::new(*q);
            let mut driver =
                CompanionFirstCenter::new(q.clone(), &format!("limited_cusp_r{oi}_{ci}"), false)
                    .unwrap();
            for _ in 0..100 {
                match driver.advance(&mut b).unwrap() {
                    RecursiveAdvance::Progress => {}
                    RecursiveAdvance::Complete => break,
                    RecursiveAdvance::Incomplete { reason } => {
                        assert_eq!(reason, "coefficient mark LCM");
                        let accepted = driver.accepted_child_steps();
                        assert!(matches!(driver.outcome(), CompanionCenterOutcome::Pending));
                        assert!(matches!(
                            driver.advance(&mut b).unwrap(),
                            RecursiveAdvance::Incomplete {
                                reason: "coefficient mark LCM"
                            }
                        ));
                        assert_eq!(driver.accepted_child_steps(), accepted);
                        let child = q.coefficient().coefficient();
                        let child_order = produce_ordinary_contact_cover(
                            q.contact().contact().clone(),
                            Arc::new(child.ideal().clone()),
                            &mut b,
                        )
                        .unwrap();
                        let OrderProduction::ContactCover(child_order) = child_order else {
                            panic!()
                        };
                        println!(
                            "LIMIT x^3+y^5 childmark{} childmax{} max_mark{} accepted{} retained_pending",
                            child.mark(),
                            child_order.algebraic_maximum_order(),
                            b.limits.max_mark,
                            accepted
                        );
                        assert_eq!(child.mark(), 6);
                        assert_eq!(child_order.algebraic_maximum_order(), 10);
                        return;
                    }
                }
            }
        }
    }
    panic!("expected exact retained resource frontier");
}

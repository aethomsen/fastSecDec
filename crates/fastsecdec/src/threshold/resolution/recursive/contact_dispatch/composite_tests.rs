use super::*;
use std::{sync::Arc, time::Instant};
use symbolica::symbol;

fn budget() -> Budget {
    Budget::new(Limits::default())
}
fn actual_partial(reverse: bool) -> Vec<Ideal> {
    let case = "deeper";
    let started = Instant::now();
    let n = 3;
    let mut b = budget();
    let ring = Arc::new(
        Ring::new(
            (0..n)
                .map(|i| symbol!(format!("bridge_{case}::x{i}")))
                .collect(),
            vec![],
        )
        .unwrap(),
    );
    let axes = (0..n).collect::<Vec<_>>();
    let local = LocalizedAlgebra::new(
        Ideal::new(ring.clone(), vec![], &mut b).unwrap(),
        axes.clone(),
        vec![],
        &mut b,
    )
    .unwrap();
    let frame = Arc::new(
        EtaleCertificate {
            source: local,
            equations: vec![],
            dependent_axes: vec![],
            free_axes: axes,
            determinant_inverse_axis: None,
        }
        .verify(&mut b)
        .unwrap(),
    );
    let x = ring.coordinate(0).unwrap();
    let y = ring.coordinate(1).unwrap();
    let z = ring.coordinate(2).unwrap();
    let g = |t: &Poly| (&t.pow(2) - &ring.one()).pow(2);
    let f = match case {
        "zero" => &x.pow(2) + &g(&y),
        "direct" => &(&x.pow(2) + &g(&y)) + &((&y + &(&ring.one() + &ring.one())) * &z.pow(2)),
        "deeper" => &(&g(&x) + &g(&y)) + &g(&z),
        _ => panic!("unknown case"),
    };
    let source =
        Arc::new(MarkedIdeal::new(Ideal::new(ring, vec![f], &mut b).unwrap(), 1, &b).unwrap());
    let mut root =
        BoundaryFreeFirstCenter::new(frame, source, &format!("bridge_{case}"), false).unwrap();
    for _ in 0..16 {
        let result = root.advance(&mut budget()).unwrap();
        println!(
            "ROOT {result:?} depth={} seconds={}",
            root.depth(),
            started.elapsed().as_secs_f64()
        );
        if !matches!(result, RecursiveAdvance::Progress) {
            break;
        }
    }
    let Some(mut d) = root
        .recursive_contact_dispatcher(format!("bridge_{case}_dispatch"), &mut budget())
        .unwrap()
    else {
        panic!("expected actual dispatch: {:#?}", root.outcome());
    };
    println!(
        "INVENTORY case={case} prefix={} dimension={} ambient={} contacts={}",
        d.cover().origin().levels().len(),
        d.cover().origin().cover().frame().free_axes().len(),
        d.ambient().len(),
        d.branches().len()
    );
    let mut chosen = None;
    for i in 0..d.branches().len() {
        accepted(
            d.advance_branch(i, &mut budget()).unwrap(),
            "candidate branch",
        );
        if d.recursive_branch(i).is_none() {
            continue;
        }
        for _ in 0..16 {
            let c = d.recursive_branch(i).unwrap();
            if c.lower().is_some() || c.child_inventory_complete() {
                break;
            }
            accepted(
                d.advance_recursive_child(i, &mut budget()).unwrap(),
                "candidate child",
            );
        }
        let c = d.recursive_branch(i).unwrap();
        println!("CANDIDATE branch={i} state={:?}", c.state());
        if c.lower().is_some() {
            chosen = Some(i);
            break;
        }
    }
    let chosen = chosen.expect("actual finite lower source");
    let mut grandchild = None;
    for i in 0..at(&d, &[chosen]).branches().len() {
        accepted(
            d.advance_nested_branch(&[chosen], i, &mut budget())
                .unwrap(),
            "find actual grandchild",
        );
        if at(&d, &[chosen]).recursive_branch(i).is_none() {
            continue;
        }
        for _ in 0..16 {
            let c = at(&d, &[chosen]).recursive_branch(i).unwrap();
            if c.lower().is_some() || c.child_inventory_complete() {
                break;
            }
            accepted(
                d.advance_nested_child(&[chosen], i, &mut budget()).unwrap(),
                "advance actual grandchild",
            );
        }
        if at(&d, &[chosen])
            .recursive_branch(i)
            .unwrap()
            .lower()
            .is_some()
        {
            grandchild = Some(i);
            break;
        }
    }
    let grandchild = grandchild.expect("genuine second finite coefficient level");
    println!("PARTIAL_SELECTED path={chosen}/{grandchild}");
    // Capture the real original source inventory before child completion.
    accepted(
        d.prepare_recursive_ascent(chosen, &mut budget()).unwrap(),
        "early partial source capture",
    );
    let stale = at(&d, &[chosen]).clone();
    drive(&mut d, &[chosen, grandchild]);
    accepted(
        d.prepare_nested_ascent(&[chosen], grandchild, &mut budget())
            .unwrap(),
        "first ancestor prepare",
    );
    let count = match at(&d, &[chosen])
        .recursive_branch(grandchild)
        .unwrap()
        .ascent()
        .unwrap()
    {
        RecursiveBranchAscent::Finite(f) => f.branches().len(),
        _ => panic!("expected first ancestor finite"),
    };
    for j in 0..count {
        accepted(
            d.advance_nested_ascent_branch(&[chosen], grandchild, j, &mut budget())
                .unwrap(),
            "first ancestor branch",
        );
        let count = match at(&d, &[chosen])
            .recursive_branch(grandchild)
            .unwrap()
            .ascent()
            .unwrap()
        {
            RecursiveBranchAscent::Finite(f) => match &f.branches()[j] {
                RecursiveAscentBranch::Components(c) => c.work().len(),
                _ => 0,
            },
            _ => 0,
        };
        for k in 0..count {
            accepted(
                d.advance_nested_ascent_open(&[chosen], grandchild, j, k, &mut budget())
                    .unwrap(),
                "first ancestor opening",
            );
        }
    }
    assert!(
        at(&d, &[chosen])
            .recursive_branch(grandchild)
            .unwrap()
            .construction_inventory_complete()
    );
    assert!(!at(&d, &[chosen]).complete_inventory());
    accepted(
        d.prepare_recursive_ascent(chosen, &mut budget()).unwrap(),
        "outer partial prepare",
    );
    accepted(
        d.advance_recursive_ascent_branch(chosen, grandchild, &mut budget())
            .unwrap(),
        "actual composite source branch",
    );
    let selected = (chosen, grandchild);
    assert_eq!(selected, (3, 4));
    check_owners(&d, selected, &stale);
    assert!(matches!(
        d.advance_composite_ascent_open(&[], selected, &[], 0, &mut no_work())
            .unwrap(),
        ContactDispatchAdvance::Incomplete { .. }
    ));
    assert!(
        node(&d, selected, &[])
            .work()
            .iter()
            .all(|w| matches!(w, CompositeLiftWork::Pending))
    );
    assert!(matches!(
        d.advance_composite_ascent_open(&[], selected, &[], usize::MAX, &mut budget()),
        Err(Error::Invalid(_))
    ));
    let mut centers = Vec::new();
    walk(&mut d, selected, &[], &mut centers, reverse);
    assert!(node(&d, (chosen, grandchild), &[]).complete_inventory());
    assert_eq!(centers.len(), 1);
    assert!(!d.complete_inventory());
    assert!(
        !d.recursive_branch(chosen)
            .unwrap()
            .construction_inventory_complete()
    );
    let pending = match d.recursive_branch(chosen).unwrap().ascent().unwrap() {
        RecursiveBranchAscent::Finite(f) => f
            .branches()
            .iter()
            .filter(|w| matches!(w, RecursiveAscentBranch::Pending))
            .count(),
        _ => 0,
    };
    assert_eq!(pending, 9);
    assert_eq!(node(&d, selected, &[]).work().len(), 20);
    let next = node(&d, selected, &[])
        .work()
        .iter()
        .find_map(|w| match w {
            CompositeLiftWork::Next(n) => Some(n),
            _ => None,
        })
        .unwrap();
    // Two actual lower contact branches include one empty contact; only the
    // nonempty component branch issues a cover node.
    assert_eq!(next.children().len(), 1);
    assert!(
        next.children()
            .iter()
            .flatten()
            .all(|c| c.complete_inventory())
    );
    assert!(
        next.children()
            .iter()
            .flatten()
            .any(|c| c.work().len() == 18)
    );
    check_owners(&d, selected, &stale);
    let original = node(&d, selected, &[]).cover().clone();
    assert!(matches!(
        d.advance_composite_ascent_open(&[], selected, &[], 0, &mut no_work())
            .unwrap(),
        ContactDispatchAdvance::Incomplete { .. }
    ));
    assert!(Arc::ptr_eq(node(&d, selected, &[]).cover(), &original));
    assert!(matches!(
        d.advance_composite_ascent_open(&[], selected, &[], 0, &mut budget())
            .unwrap(),
        ContactDispatchAdvance::Accepted
    ));
    println!(
        "ACTUAL_TWO_LEVEL_ASCENT centers={} retained_parent_pending={pending} root_contacts={} root_ambient={} full_inventory=false seconds={}",
        centers.len(),
        d.branches().len(),
        d.ambient().len(),
        started.elapsed().as_secs_f64()
    );
    centers
}
fn at<'a>(root: &'a FiniteContactDispatcher, path: &[usize]) -> &'a FiniteContactDispatcher {
    path.iter().fold(root, |d, i| {
        d.recursive_branch(*i).unwrap().lower().unwrap()
    })
}
fn accepted(r: ContactDispatchAdvance, label: &str) {
    assert!(
        !matches!(r, ContactDispatchAdvance::Incomplete { .. }),
        "{label}: {r:?}"
    );
}
fn drive(root: &mut FiniteContactDispatcher, path: &[usize]) {
    assert!(path.len() < 4);
    let (ambient, branches) = (
        at(root, path).ambient().len(),
        at(root, path).branches().len(),
    );
    println!("DRIVE_BEGIN path={path:?} ambient={ambient} branches={branches}");
    for i in 0..ambient {
        let frame = at(root, path).cover().origin().auxiliary().frame();
        println!(
            "PHASE_BEGIN ambient path={path:?} index={i} vars={} equations={} guards={} free={}",
            frame.local().ring().len(),
            frame.local().ideal().generators().len(),
            frame.local().guards().len(),
            frame.free_axes().len()
        );
        let t = Instant::now();
        accepted(
            root.advance_nested_ambient(path, i, &mut budget()).unwrap(),
            "ambient",
        );
        println!(
            "PHASE_END ambient path={path:?} index={i} seconds={}",
            t.elapsed().as_secs_f64()
        );
    }
    for i in 0..branches {
        println!("PHASE_BEGIN branch path={path:?} index={i}");
        let t = Instant::now();
        accepted(
            root.advance_nested_branch(path, i, &mut budget()).unwrap(),
            "branch",
        );
        println!(
            "PHASE_END branch path={path:?} index={i} seconds={}",
            t.elapsed().as_secs_f64()
        );
        match &at(root, path).branches()[i] {
            ContactBranchWork::Empty(_) => continue,
            ContactBranchWork::Components(f) => {
                let count = f.work().len();
                for j in 0..count {
                    println!("PHASE_BEGIN component path={path:?} branch={i} index={j}");
                    let t = Instant::now();
                    accepted(
                        root.advance_nested_component(path, i, j, &mut budget())
                            .unwrap(),
                        "component",
                    );
                    println!("PHASE_END component seconds={}", t.elapsed().as_secs_f64());
                }
                continue;
            }
            ContactBranchWork::Recursive(_) => {}
            ContactBranchWork::Pending => panic!("accepted pending"),
        }
        for _ in 0..16 {
            let c = at(root, path).recursive_branch(i).unwrap();
            if c.child_inventory_complete() || c.lower().is_some() {
                break;
            }
            accepted(
                root.advance_nested_child(path, i, &mut budget()).unwrap(),
                "child",
            );
        }
        let c = at(root, path).recursive_branch(i).unwrap();
        println!("DRIVE_CHILD path={path:?}/{i} state={:?}", c.state());
        if c.lower().is_some() {
            let mut next = path.to_vec();
            next.push(i);
            drive(root, &next);
        }
        accepted(
            root.prepare_nested_ascent(path, i, &mut budget()).unwrap(),
            "prepare ascent",
        );
        let count = match at(root, path)
            .recursive_branch(i)
            .unwrap()
            .ascent()
            .unwrap()
        {
            RecursiveBranchAscent::Finite(f) => f.branches().len(),
            _ => 0,
        };
        for j in 0..count {
            accepted(
                root.advance_nested_ascent_branch(path, i, j, &mut budget())
                    .unwrap(),
                "ascent branch",
            );
            let count = match at(root, path)
                .recursive_branch(i)
                .unwrap()
                .ascent()
                .unwrap()
            {
                RecursiveBranchAscent::Finite(f) => match &f.branches()[j] {
                    RecursiveAscentBranch::Components(c) => c.work().len(),
                    _ => 0,
                },
                _ => 0,
            };
            for k in 0..count {
                accepted(
                    root.advance_nested_ascent_open(path, i, j, k, &mut budget())
                        .unwrap(),
                    "ascent open",
                );
            }
        }
        assert!(
            at(root, path)
                .recursive_branch(i)
                .unwrap()
                .construction_inventory_complete()
        );
    }
    assert!(at(root, path).complete_inventory());
    println!("DRIVE_COMPLETE path={path:?} ambient={ambient} branches={branches}");
}

fn node<'a>(
    d: &'a FiniteContactDispatcher,
    selected: (usize, usize),
    path: &[(usize, usize)],
) -> &'a CompositeCoverLift {
    let RecursiveBranchAscent::Finite(f) =
        d.recursive_branch(selected.0).unwrap().ascent().unwrap()
    else {
        panic!("no finite parent")
    };
    let RecursiveAscentBranch::Composite(c) = &f.branches()[selected.1] else {
        panic!("no composite")
    };
    let mut c = c.as_ref();
    for &(i, j) in path {
        let CompositeLiftWork::Next(n) = &c.work()[i] else {
            panic!("no next")
        };
        c = n.children()[j].as_deref().unwrap();
    }
    c
}
fn walk(
    d: &mut FiniteContactDispatcher,
    selected: (usize, usize),
    path: &[(usize, usize)],
    centers: &mut Vec<Ideal>,
    reverse: bool,
) {
    let count = node(d, selected, path).work().len();
    let mut indices = (0..count).collect::<Vec<_>>();
    if reverse {
        indices.reverse();
    }
    for i in indices {
        println!("COMPOSITE_BEGIN path={path:?} open={i}/{count}");
        accepted(
            d.advance_composite_ascent_open(&[], selected, path, i, &mut budget())
                .unwrap(),
            "composite opening",
        );
        match &node(d, selected, path).work()[i] {
            CompositeLiftWork::Next(n) => {
                let nc = n.children().len();
                for j in 0..nc {
                    accepted(
                        d.prepare_composite_ascent_child(
                            &[],
                            selected,
                            path,
                            (i, j),
                            &mut budget(),
                        )
                        .unwrap(),
                        "composite next cover",
                    );
                    let mut next = path.to_vec();
                    next.push((i, j));
                    walk(d, selected, &next, centers, reverse);
                }
            }
            CompositeLiftWork::Center(c) => {
                assert_eq!(c.source().mark(), 1);
                assert!(!c.ancestors().is_empty());
                assert!(
                    c.history()
                        .same_root(c.branch().owner().origin().auxiliary().history())
                );
                assert_eq!(
                    c.history().stage(),
                    c.branch().owner().origin().auxiliary().history().stage()
                );
                assert!(Arc::ptr_eq(c.lower_owner(), at(d, &[selected.0]).cover()));
                assert!(
                    c.ancestors()
                        .iter()
                        .all(|a| a.presentation().original_source().mark() > 0)
                );
                centers.push(c.ideal().clone());
                println!(
                    "TWO_LEVEL_CENTER normals={} ancestor_layers={}",
                    c.normals().len(),
                    c.ancestors().len()
                );
            }
            CompositeLiftWork::Pending => panic!("pending accepted"),
            _ => {}
        }
    }
    assert!(node(d, selected, path).complete_inventory());
}

fn no_work() -> Budget {
    Budget::new(Limits {
        max_operations: 0,
        ..Limits::default()
    })
}
fn check_owners(
    d: &FiniteContactDispatcher,
    selected: (usize, usize),
    stale: &FiniteContactDispatcher,
) {
    let RecursiveBranchAscent::Finite(f) =
        d.recursive_branch(selected.0).unwrap().ascent().unwrap()
    else {
        panic!("finite")
    };
    let live = at(d, &[selected.0]);
    assert!(!f.lower().complete_inventory());
    assert!(!f.complete_inventory());
    assert!(matches!(
        f.verify_live(stale, &mut budget()),
        Err(Error::Invalid(_))
    ));
    let mut forged = f.as_ref().clone();
    forged.child_source = Arc::new(f.child_source.as_ref().clone());
    assert!(matches!(
        forged.verify_live(live, &mut budget()),
        Err(Error::Invalid(_))
    ));
    let mut omitted = f.as_ref().clone();
    omitted.branches.pop();
    assert!(matches!(
        omitted.verify_live(live, &mut budget()),
        Err(Error::Invalid(_))
    ));
    let mut retry = f.as_ref().clone();
    let retained = retry.lower.clone();
    assert!(matches!(
        retry
            .advance_branch(selected.1, live, &mut no_work())
            .unwrap(),
        ContactDispatchAdvance::Incomplete { .. }
    ));
    assert!(Arc::ptr_eq(&retry.lower, &retained));
    assert_eq!(retry.branches.len(), f.branches.len());
    assert!(matches!(
        retry
            .advance_branch(selected.1, live, &mut budget())
            .unwrap(),
        ContactDispatchAdvance::Accepted
    ));
}

#[test]
fn recursive_composite_partial_actual_two_level_and_retry() {
    let first = actual_partial(false);
    let reverse = actual_partial(true);
    assert_eq!(
        first, reverse,
        "same source/namespace gives same exact center ideal after reordered cover traversal"
    );
}

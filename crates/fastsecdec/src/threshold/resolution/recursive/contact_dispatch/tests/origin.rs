use super::*;
#[test]
fn contact_dispatch_preserves_actual_origin_and_prefix_without_center_authority() {
    let mut b = Budget::new(Limits::default());
    let mut driver = live(&mut b);
    assert!(driver.contact_dispatch_origin(&mut b).unwrap().is_none());
    for _ in 0..10 {
        match driver.advance(&mut b).unwrap() {
            RecursiveAdvance::Progress => {}
            RecursiveAdvance::Complete => break,
            _ => panic!("unexpected native stop"),
        }
    }
    let source = driver.contact_dispatch_origin(&mut b).unwrap().unwrap();
    assert_eq!(source.levels().len(), 1);
    assert_eq!(source.cover().candidates().len(), 4);
    assert_eq!(source.cover().algebraic_maximum_order(), 4);
    assert_eq!(source.presentation().original_source().mark(), 2);
    assert!(Arc::ptr_eq(
        source.levels()[0].source(),
        source.auxiliary().source()
    ));
    assert!(Arc::ptr_eq(
        source.cover().frame(),
        source.levels()[0].contact().contact()
    ));
    let accepted = driver.accepted_steps();
    let mut empty = Budget::new(Limits {
        max_operations: 0,
        ..Limits::default()
    });
    assert!(matches!(
        driver.contact_dispatch_origin(&mut empty),
        Err(Error::ResourceIncomplete(_))
    ));
    assert_eq!(accepted, driver.accepted_steps());
    assert!(matches!(
        driver.outcome(),
        RecursiveOutcome::MultipleContactOpensRequired { .. }
    ));
    assert!(matches!(
        ContactDispatchOrigin::prepare(
            source.auxiliary().clone(),
            &[],
            source.cover().clone(),
            source.presentation().clone(),
            &mut b
        ),
        Err(Error::Invalid(_))
    ));
    let duplicate = vec![source.levels()[0].clone(), source.levels()[0].clone()];
    assert!(matches!(
        ContactDispatchOrigin::prepare(
            source.auxiliary().clone(),
            &duplicate,
            source.cover().clone(),
            source.presentation().clone(),
            &mut b
        ),
        Err(Error::Invalid(_))
    ));
    let mut foreign = live(&mut b);
    for _ in 0..10 {
        if matches!(foreign.advance(&mut b).unwrap(), RecursiveAdvance::Complete) {
            break;
        }
    }
    let foreign = foreign.contact_dispatch_origin(&mut b).unwrap().unwrap();
    assert!(matches!(
        ContactDispatchOrigin::prepare(
            source.auxiliary().clone(),
            source.levels(),
            foreign.cover().clone(),
            foreign.presentation().clone(),
            &mut b
        ),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn contact_dispatch_complete_cover_lifts_to_original_history() {
    let mut b = Budget::new(Limits::default());
    let mut driver = live(&mut b);
    for _ in 0..10 {
        if matches!(driver.advance(&mut b).unwrap(), RecursiveAdvance::Complete) {
            break;
        }
    }
    let origin = driver.contact_dispatch_origin(&mut b).unwrap().unwrap();
    let cover = ContactDispatchCover::prepare(origin.clone(), &mut b).unwrap();
    assert_eq!(cover.current().opens().len(), 8);
    assert_eq!(cover.ancestors().len(), 1);
    assert_eq!(cover.physical_cover().opens().len(), 9);
    assert!(Arc::ptr_eq(
        cover.physical_cover().algebra(),
        origin.auxiliary().frame().local()
    ));
    assert!(Arc::ptr_eq(
        cover.ancestors()[0].source(),
        origin.levels()[0].source()
    ));
    let mut admitted = 0;
    for i in 0..cover.physical_cover().opens().len() {
        match cover
            .localize(i, &format!("contact_dispatch_full_{i}"), &mut b)
            .unwrap()
        {
            CoverLocalizationProduction::Complete(o) => {
                assert!(o.history().same_root(origin.auxiliary().history()));
                assert_eq!(o.history().stage(), 0);
                assert!(o.history().births().is_empty());
                assert!(Arc::ptr_eq(o.open().source(), origin.auxiliary().history()));
                admitted += 1;
            }
            CoverLocalizationProduction::Empty { .. } => admitted += 1,
            CoverLocalizationProduction::Incomplete { reason, .. } => {
                panic!("unexpected incomplete full cover: {reason}")
            }
        }
    }
    assert_eq!(admitted, 9);
    let mut empty = Budget::new(Limits {
        max_operations: 0,
        ..Limits::default()
    });
    assert!(matches!(
        ContactDispatchCover::prepare(origin.clone(), &mut empty),
        Err(Error::ResourceIncomplete(_))
    ));
    assert_eq!(origin.levels().len(), 1);
}

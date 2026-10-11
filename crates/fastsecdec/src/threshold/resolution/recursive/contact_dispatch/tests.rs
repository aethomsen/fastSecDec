use super::*;
use symbolica::symbol;
fn live(b: &mut Budget) -> BoundaryFreeFirstCenter {
    live_order(b, false)
}
fn live_order(b: &mut Budget, reverse: bool) -> BoundaryFreeFirstCenter {
    let r = Arc::new(
        Ring::new(
            vec![symbol!("dispatch_test::x"), symbol!("dispatch_test::y")],
            vec![],
        )
        .unwrap(),
    );
    let local = LocalizedAlgebra::new(
        Ideal::new(r.clone(), vec![], b).unwrap(),
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
    let x = r.coordinate(0).unwrap();
    let y = r.coordinate(1).unwrap();
    let ym = &y - &r.one();
    let y3 = b.power(&y, 3).unwrap();
    let ym4 = b.power(&ym, 4).unwrap();
    let g = b.mul(&y3, &ym4).unwrap();
    let p = &b.power(&x, 2).unwrap() + &g;
    let source = Arc::new(MarkedIdeal::new(Ideal::new(r, vec![p], b).unwrap(), 1, b).unwrap());
    BoundaryFreeFirstCenter::new(frame, source, "contact_dispatch_actual", reverse).unwrap()
}

mod ascent;
mod driver;
mod origin;

#[test]
fn contact_dispatch_public_owners_preserve_native_send_sync() {
    fn check<T: Send + Sync>() {}
    check::<DispatchedContactCenter>();
    check::<FiniteContactDispatcher>();
    check::<EmbeddedMonomialCenter>();
}

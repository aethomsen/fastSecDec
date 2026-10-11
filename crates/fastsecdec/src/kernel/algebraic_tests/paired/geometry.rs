use super::*;

#[test]
fn unexpanded_three_cell_partition_and_positive_measure() {
    for affine in [false, true] {
        let f = family(0, affine);
        assert_eq!(f.owner().cells().len(), 3);
        assert_eq!(f.charts().len(), 4);
        assert_eq!(f.roots().len(), 2);
        assert_eq!(f.strip().upper(), Some(&Rational::zero()));
        let cells = f
            .charts()
            .iter()
            .map(|c| c.geometry().source().cell_index())
            .collect::<Vec<_>>();
        assert_eq!(cells[1], cells[2]);
        assert_ne!(cells[0], cells[1]);
        assert_ne!(cells[2], cells[3]);
        let continued = f.continue_symbolically(&options(2)).unwrap();
        let e = Atom::var(f.regulators()[0]);
        let sx = Atom::var(f.coordinates()[0]);
        let t = Atom::var(f.coordinates()[1]);
        let mut expressions = Vec::new();
        for c in f.charts() {
            let derivative = c.image().derivative(f.coordinates()[1]);
            let signed = if c.geometry().reversed() {
                -derivative
            } else {
                derivative
            };
            assert!((signed - c.positive_measure()).expand().is_zero());
            let raw: Atom = c
                .terms()
                .iter()
                .map(|term| {
                    term.prefactor()
                        * term.regular()
                        * term
                            .powers()
                            .iter()
                            .zip(f.coordinates())
                            .map(|(q, s)| Atom::var(*s).pow(q))
                            .product::<Atom>()
                })
                .sum();
            let p = c.image().pow(6) - c.image() + (&sx + Atom::one()) / Atom::num(16);
            expressions.push(raw.replace(e.clone()).with(Atom::num(-2)));
            expressions.push(c.positive_measure() * p);
        }
        // Native polynomial primitive, independent of the normalized secant factors.
        let a = symgcad::algebra::Algebra::new(&["x".into(), "y".into()]).unwrap();
        let p = a.parse("y^6-y+(1+x)/16").unwrap();
        let primitive = p
            .integrate(1)
            .to_expression()
            .replace(Atom::var(Symbol::parse("x", "symgcad").unwrap()))
            .with(sx.clone())
            .replace(Atom::var(Symbol::parse("y", "symgcad").unwrap()))
            .with(t.clone());
        let mut partition = Atom::zero();
        for c in f.charts() {
            let lo = c.image().replace(t.clone()).with(Atom::zero());
            let hi = c.image().replace(t.clone()).with(Atom::one());
            let value =
                primitive.replace(t.clone()).with(hi) - primitive.replace(t.clone()).with(lo);
            partition += if c.geometry().reversed() {
                -value
            } else {
                value
            };
        }
        assert!(
            (partition
                - (primitive.replace(t.clone()).with(Atom::one())
                    - primitive.replace(t.clone()).with(Atom::zero())))
            .expand()
            .is_zero()
        );
        let primitive = p.integrate(1).integrate(0);
        let integral = &(&primitive
            .replace(0, &Rational::one())
            .replace(1, &Rational::one())
            - &primitive
                .replace(0, &Rational::zero())
                .replace(1, &Rational::one()))
            - &(&primitive
                .replace(0, &Rational::one())
                .replace(1, &Rational::zero())
                - &primitive
                    .replace(0, &Rational::zero())
                    .replace(1, &Rational::zero()));
        assert_eq!(
            Rational::try_from(integral.to_expression().as_view()).unwrap(),
            Rational::from((-59, 224))
        );
        let exact = Atom::evaluator_multiple(&expressions, &[sx, t])
            .function_map(continued.functions().clone())
            .optimization_settings(OptimizationSettings::default().cores(1))
            .build()
            .unwrap();
        let mut eval = f.callback_scope().enter(53, || {
            exact.map_coeff(&|c| Complex::new(c.re.to_f64(), c.im.to_f64()))
        });
        for x in [0., 0.17, 0.5, 1.] {
            for t in [0.01, 0.25, 0.75, 1.] {
                let mut out = vec![Complex::new(0., 0.); 8];
                regular::callback::attempt(|| {
                    eval.evaluate(&[Complex::new(x, 0.), Complex::new(t, 0.)], &mut out)
                })
                .unwrap();
                for pair in out.chunks_exact(2) {
                    assert!(
                        (pair[0].re - pair[1].re).abs() < 2e-13 && pair[0].im.abs() < 2e-13,
                        "{pair:?}"
                    );
                }
            }
        }
    }
}
#[test]
fn paired_admission_refuses_wrong_branches_limits_and_cancel() {
    let (o, u) = source(0);
    let mut p = proposals();
    p.swap(0, 1);
    assert!(NormalizedFamily::admit_pair(o.clone(), u.clone(), p, limits()).is_err());
    let mut p = proposals();
    p[1].upper = Rational::one();
    assert!(NormalizedFamily::admit_pair(o.clone(), u.clone(), p, limits()).is_err());
    let mut l = limits();
    l.geometry.max_terms = 1;
    assert!(matches!(
        NormalizedFamily::admit_pair(o.clone(), u.clone(), proposals(), l),
        Err(secant::Error::ResourceIncomplete(_))
    ));
    let mut l = limits();
    l.cancellation = Some(Arc::new(AtomicBool::new(true)));
    assert!(matches!(
        NormalizedFamily::admit_pair(o, u, proposals(), l),
        Err(secant::Error::Cancelled)
    ));
}
#[test]
fn paired_helper_roles_and_original_order_are_fenced() {
    let (owner, units) = source_config(0, true, None);
    assert!(
        matches!(NormalizedFamily::admit_pair(owner,units,proposals(),limits()),Err(secant::Error::Unsupported(message)) if message.contains("original integration-axis order"))
    );
    for helper in [
        symbol!("crate::secant_endpoint::lower_root"),
        symbol!("crate::secant_endpoint::upper_root"),
        symbol!("crate::secant_endpoint::numerator"),
    ] {
        for used in [false, true] {
            let (owner, units) = source_config(0, false, Some((helper, used)));
            assert!(
                matches!(NormalizedFamily::admit_pair(owner,units,proposals(),limits()),Err(secant::Error::Invalid(message)) if message.contains("collides"))
            );
        }
    }
}

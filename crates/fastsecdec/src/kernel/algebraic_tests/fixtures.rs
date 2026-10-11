use super::*;
pub(super) fn family(shifted: bool) -> secant::NormalizedFamily {
    family_digits(shifted, 20)
}
pub(super) fn family_digits(shifted: bool, digits: u64) -> secant::NormalizedFamily {
    family_recipe(shifted, digits, false)
}
pub(super) fn family_recipe(shifted: bool, digits: u64, affine: bool) -> secant::NormalizedFamily {
    family_version(shifted, digits, if affine { 2 } else { 1 })
}
pub(super) fn family_version(shifted: bool, digits: u64, version: u32) -> secant::NormalizedFamily {
    let (x, y, e, s, t) = symbol!(
        "kernel_root_c::x",
        "kernel_root_c::y",
        "kernel_root_c::e",
        "kernel_root_c::s",
        "kernel_root_c::t"
    );
    let delta = Rational::from((1, 10)).pow(digits);
    let polynomial = Atom::var(y).pow(5) + Atom::var(y) - Atom::var(x)
        + if shifted {
            Atom::num(delta.clone()) - Atom::one()
        } else {
            Atom::zero()
        };
    let power = -Atom::var(e) - if shifted { Atom::zero() } else { Atom::one() };
    let input = ParametricIntegrand::new(
        vec![x, y],
        e,
        ParametricDomain::UnitCube,
        vec![ParametricTerm::new(
            Atom::one(),
            vec![Atom::zero(); 2],
            vec![
                PolynomialFactor::new(polynomial, power, FactorRole::Singularity)
                    .with_semantics(FactorSemantics::Causal),
            ],
        )],
    )
    .unwrap();
    let mut limits = GcadRequest::default_limits();
    limits.wall_time_secs = 30.;
    limits.memory_mib = 1024;
    let request = GcadRequest::unit_cube(
        &input,
        GcadKinematics::default(),
        SolverOptions {
            order: vec!["v0".into(), "v1".into()],
            ..Default::default()
        },
        limits,
    )
    .unwrap();
    let owner = Arc::new(request.solve_verified().unwrap());
    let mut limits = secant::Limits::default();
    if shifted {
        limits.unit_margin = &delta / &Rational::from(4);
    }
    let admit = match version {
        1 => secant::NormalizedFamily::admit,
        2 => secant::NormalizedFamily::admit_affine,
        3 => secant::NormalizedFamily::admit_affine_shifted,
        _ => unreachable!("test recipe"),
    };
    admit(owner, vec![s, t], limits).unwrap()
}
pub(super) fn make_kernel(
    family: &secant::NormalizedFamily,
    expression: &Atom,
    functions: &symbolica::evaluate::FunctionMap,
    profiles: Vec<generation::EndpointProfileRow>,
    orders: &[i32],
    backend: EvaluatorBackend,
) -> SectorKernel {
    make_kernel_with_policy(
        family,
        expression,
        functions,
        profiles,
        orders,
        backend,
        &PrecisionPolicy::default(),
    )
}
pub(super) fn make_kernel_with_policy(
    family: &secant::NormalizedFamily,
    expression: &Atom,
    functions: &symbolica::evaluate::FunctionMap,
    profiles: Vec<generation::EndpointProfileRow>,
    orders: &[i32],
    backend: EvaluatorBackend,
    precision: &PrecisionPolicy,
) -> SectorKernel {
    let expanded = generation::threshold_expand_vector(
        expression,
        family.coordinates(),
        family.regulators()[0],
        *orders.last().unwrap(),
    )
    .unwrap();
    let vector = PreparedCoefficientVector {
        coordinates: family.coordinates().to_vec(),
        coefficients: orders
            .iter()
            .map(|order| {
                expanded
                    .get(order)
                    .cloned()
                    .unwrap_or_else(|| AliasedAtom::from(Atom::zero()))
            })
            .collect(),
        functions: Arc::new(functions.clone()),
        endpoint_profiles: profiles,
    };
    let settings = CompilationSettings {
        backend,
        ..Default::default()
    };
    family.callback_scope().enter(53, || {
        let program = vector.build(settings).unwrap();
        SectorKernel::from_program_with_backend(program, precision, true, backend).unwrap()
    })
}

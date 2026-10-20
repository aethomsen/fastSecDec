//! Native graph inputs and independent references; no generation or sampling.
use idenso::tensor::SymbolicTensor;
use std::{
    collections::{BTreeMap, HashMap},
    error::Error,
    sync::Arc,
};

use fastsecdec::{
    Atom, AtomCore, Kinematics, Model,
    input::{GraphIntegral, default_algebra_settings},
    parametric::{ParametricIntegrand, ScalarParametricIntegral},
    status::CoefficientComponent,
};
use feynkit_graph::symbols;
use oneloop::{EvaluationBackend, ScalarIntegral};
use serde::{Deserialize, Serialize};
use symbolica::{
    atom::Symbol, domains::float::Complex, parse, symbol, transcendental::TranscendentalFunctions,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub const CASES: [&str; 4] = ["triangle", "box", "sunrise", "kite"];
const MODEL: &str = include_str!("../../../../examples/contour/scalar_benchmarks/model.json");
const KITE: &str = include_str!("../../../../examples/contour/scalar_benchmarks/kite.dot");
const KITE_REFERENCE: &str = "https://github.com/gudrunhe/secdec/blob/2b3287ecd59436147350ae630a6fdd19eaba9097/examples/bubble2L_largem_ebr/integrate_bubble2L_full.py";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reference {
    /// Each value is [real, imaginary]; absent orders below zero vanish.
    pub coefficients: BTreeMap<i32, [f64; 2]>,
    pub method: String,
    pub source_url: String,
    /// Series truncation only, not a bound on binary64 evaluation roundoff.
    pub absolute_truncation_bound: Option<f64>,
}

impl Reference {
    pub fn values_for_layout(
        &self,
        orders: &[i32],
        components: &[CoefficientComponent],
    ) -> Result<Vec<f64>> {
        if orders.len() != components.len() || orders.iter().any(|order| *order > 0) {
            return Err(
                "reference supports complete Laurent layouts only through order zero".into(),
            );
        }
        Ok(orders
            .iter()
            .zip(components)
            .map(|(order, component)| {
                let value = self.coefficients.get(order).copied().unwrap_or([0., 0.]);
                match component {
                    CoefficientComponent::Real => value[0],
                    CoefficientComponent::Imag => value[1],
                }
            })
            .collect())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Admission {
    pub case: String,
    pub case_index: usize,
    pub loops: usize,
    pub projective_dimension: usize,
    pub point: String,
    pub powers: Vec<u32>,
    pub denominators: Vec<String>,
    pub u: String,
    pub f: String,
    pub prefactor: String,
    pub u_exponent: String,
    pub f_exponent: String,
    pub measure_multiplier: String,
    pub native_scalar_numerator_verified: bool,
    pub normalization: String,
}

pub struct Fixture {
    pub graph: GraphIntegral,
    pub input: ParametricIntegrand,
    pub admission: Admission,
    pub reference: Reference,
}

fn epsilon() -> Symbol {
    symbol!("contour_scalar::eps")
}

fn dimension() -> Atom {
    parse!("4-2*contour_scalar::eps")
}

fn kin() -> Result<Kinematics> {
    Ok(Kinematics::in_dimension(&parse!("contour_scalar::D"))?)
}

fn graph(dot: &str, kinematics: &Kinematics, mass: i64) -> Result<GraphIntegral> {
    Ok(
        GraphIntegral::from_dot(Arc::new(Model::from_json(MODEL)?), dot, kinematics)?
            .with_scalar_values(&BTreeMap::from([(symbol!("UFO::mt"), Atom::num(mass))]))?,
    )
}

fn equal(actual: &Atom, expected: &Atom, label: &str) -> Result<()> {
    if (actual - expected).expand().is_zero() {
        Ok(())
    } else {
        Err(format!("native {label} mismatch: {actual} != {expected}").into())
    }
}

fn master_measure() -> Atom {
    // mu^2=1; the multiplier is explicitly part of the generated integral.
    parse!(
        "gamma(1-2*contour_scalar::eps)/(gamma(1+contour_scalar::eps)*gamma(1-contour_scalar::eps)^2)"
    )
}

fn one_loop_reference(family: ScalarIntegral, arguments: &[i64]) -> Result<Reference> {
    let arguments = arguments
        .iter()
        .map(|value| Complex::new(*value as f64, 0.))
        .collect::<Vec<_>>();
    let mut output = [Complex::new(0., 0.); 3];
    oneloop::evaluate_with_backend(
        family,
        &arguments,
        &mut output,
        EvaluationBackend::Expression,
    )?;
    if output
        .iter()
        .any(|value| !value.re.is_finite() || !value.im.is_finite())
    {
        return Err("native OneLOop reference is nonfinite".into());
    }
    Ok(Reference {
        coefficients: [0, -1, -2]
            .into_iter()
            .zip(output.map(|value| [value.re, value.im]))
            .collect(),
        method: "Native OneLOop EvaluationBackend::Expression, [finite,pole,double-pole]".into(),
        source_url: "https://arxiv.org/abs/1007.4716".into(),
        absolute_truncation_bound: None,
    })
}

fn complex(expression: &Atom) -> Result<Complex<f64>> {
    let value = expression.evaluate(&HashMap::<Atom, Complex<f64>>::new())?;
    if !value.re.is_finite() || !value.im.is_finite() {
        return Err("native analytic reference is nonfinite".into());
    }
    Ok(value)
}

fn sunrise_reference() -> Result<Reference> {
    // -(-s-i0)^(1-2eps), s=1, equals exp(2*i*pi*eps).
    // The normalized measure has no implicit exp(gamma_E*eps) factors.
    let phase = (Atom::num(2) * Atom::i() * Atom::var(Symbol::PI) * Atom::var(epsilon())).exp();
    let value = phase
        * parse!(
            "gamma(-1+2*contour_scalar::eps)*gamma(1-contour_scalar::eps)^3/gamma(3-3*contour_scalar::eps)"
        );
    let series = value.series(epsilon(), 0, 0)?;
    let mut coefficients = BTreeMap::new();
    for order in [-2_i32, -1, 0] {
        let coefficient = complex(
            &series
                .coefficient(i64::from(order).into())
                .ok_or("native sunrise series does not cover a requested coefficient")?,
        )?;
        coefficients.insert(order, [coefficient.re, coefficient.im]);
    }
    Ok(Reference {
        coefficients,
        method: "Native Symbolica Laurent series of the composed massless bubble identity; log(-1-i0)=-i*pi".into(),
        source_url: "https://www-library.desy.de/preparch/desy/proc/ali/proc/grozin_andrey/grozin_andrey.pdf".into(),
        absolute_truncation_bound: None,
    })
}

fn kite_series() -> (Atom, Atom, Atom) {
    let s = Atom::num((3, 1000));
    let q = -&s / 4;
    let mut f0 = Atom::Zero;
    let mut f1 = Atom::Zero;
    // Published convergent series, evaluated by native Gamma/Atom arithmetic.
    for n in 0..=8_i64 {
        let ratio = Atom::num(n + 1).gamma() * Atom::num((3, 2)).gamma()
            / Atom::num((2 * n + 3, 2)).gamma();
        let term = ratio * q.pow(n);
        f0 += &term * Atom::num((3, 2)) / Atom::num((n + 1) * (n + 1));
        f1 -= term / Atom::num(2 * (n + 1));
    }
    (s, f0, f1)
}

fn kite_reference() -> Result<Reference> {
    let (s, f0, f1) = kite_series();
    let logarithm = s.log() - Atom::i() * Atom::var(Symbol::PI);
    let value = complex(&(f0 + f1 * logarithm))?;
    let q_abs = 3_f64 / 4000.;
    let log_abs = (0.003_f64.ln().powi(2) + std::f64::consts::PI.powi(2)).sqrt();
    // Both positive coefficient ratios decrease. This bounds the remaining
    // absolute geometric tail; it does not claim to bound numeric roundoff.
    let bound = (3. + log_abs) / 2. * q_abs.powi(9) / (1. - q_abs);
    if !(bound.is_finite() && bound < 1e-26) {
        return Err("kite reference truncation bound was not established".into());
    }
    Ok(Reference {
        coefficients: BTreeMap::from([(-2, [0., 0.]), (-1, [0., 0.]), (0, [value.re, value.im])]),
        method: "Fleischer-Smirnov-Tarasov zero-threshold series n=0..8, native Gamma evaluation, exact zero-regulator log(s)-i*pi; upstream printed cross-check uses s+i*1e-15".into(),
        source_url: KITE_REFERENCE.into(),
        absolute_truncation_bound: Some(bound),
    })
}

pub fn build(name: &str) -> Result<Fixture> {
    let case_index = CASES
        .iter()
        .position(|case| *case == name)
        .ok_or("unknown scalar benchmark case")?;
    let (graph, reference, point, normalization) = match name {
        "triangle" => {
            let p = symbols::external_momentum().call(1);
            let q = symbols::external_momentum().call(2);
            let kinematics = kin()?
                .with_mass_squared(&p, Atom::Zero)?
                .with_mass_squared(&q, Atom::Zero)?
                .with_scalar_product(&p, &q, Atom::num((5, 2)))?;
            (
                graph(
                    include_str!("../../../../examples/graphs/triangle.dot"),
                    &kinematics,
                    1,
                )?
                .with_measure_multiplier(master_measure()),
                one_loop_reference(ScalarIntegral::C0, &[0, 0, 5, 1, 1, 1, 1])?,
                "C0(0,0,5;1,1,1), mu_squared=1",
                "normalized loop measure multiplied by 1/r_Gamma",
            )
        }
        "box" => {
            let momenta = (0..4)
                .map(|i| symbols::external_momentum().call(i))
                .collect::<Vec<_>>();
            let twice_gram = [
                [0, 5, -4, -1],
                [5, 0, -1, -4],
                [-4, -1, 0, 5],
                [-1, -4, 5, 0],
            ];
            let mut kinematics = kin()?.with_momenta(momenta.clone())?;
            for i in 0..4 {
                for j in i..4 {
                    kinematics = kinematics.with_scalar_product(
                        &momenta[i],
                        &momenta[j],
                        Atom::num((twice_gram[i][j], 2)),
                    )?;
                }
            }
            (
                graph(
                    include_str!("../../../../examples/graphs/box.dot"),
                    &kinematics,
                    1,
                )?
                .with_measure_multiplier(master_measure()),
                one_loop_reference(ScalarIntegral::D0, &[0, 0, 0, 0, 5, -1, 1, 1, 1, 1, 1])?,
                "D0(0,0,0,0,5,-1;1,1,1,1), mu_squared=1",
                "normalized loop measure multiplied by 1/r_Gamma",
            )
        }
        "sunrise" => {
            let kinematics =
                kin()?.with_mass_squared(&symbols::external_momentum().call(0), Atom::one())?;
            let source = graph(
                include_str!("../../../../examples/graphs/sunset_2loop_numerator.dot"),
                &kinematics,
                0,
            )?;
            let scalar = source
                .diagram()
                .as_ref()
                .clone()
                .with_numerator(SymbolicTensor::infer(Atom::one())?)?;
            let graph = GraphIntegral::new(Arc::new(scalar), source.family().kinematics())?
                .with_scalar_values(&BTreeMap::from([(symbol!("UFO::mt"), Atom::Zero)]))?;
            (
                graph,
                sunrise_reference()?,
                "p_squared=1, three massless propagators",
                "prod d^Dk/(i*pi^(D/2)), no extra Euler-gamma factor",
            )
        }
        "kite" => {
            let kinematics = kin()?
                .with_mass_squared(&symbols::external_momentum().call(0), Atom::num((3, 1000)))?;
            (
                graph(KITE, &kinematics, 1)?.with_measure_multiplier(Atom::num(-1)),
                kite_reference()?,
                "p_squared=3/1000, m_squared=1",
                "prod d^Dk/(i*pi^(D/2)) times explicit -1, as the published reference",
            )
        }
        _ => unreachable!(),
    };
    equal(
        &graph.scalar_numerator(&default_algebra_settings())?,
        &Atom::one(),
        "scalar numerator",
    )?;
    let parameters = (0..graph.powers().len())
        .map(|i| Atom::var(symbol!(format!("contour_scalar::x{i}"))))
        .collect::<Vec<_>>();
    let scalar = ScalarParametricIntegral::from_graph(&graph, parameters, dimension())?;
    if name == "kite" {
        admit_kite(&graph, &scalar)?;
    }
    if name == "sunrise" {
        equal(
            scalar.u(),
            &parse!(
                "contour_scalar::x0*contour_scalar::x1+contour_scalar::x0*contour_scalar::x2+contour_scalar::x1*contour_scalar::x2"
            ),
            "sunrise U",
        )?;
        equal(
            scalar.f(),
            &parse!("-contour_scalar::x0*contour_scalar::x1*contour_scalar::x2"),
            "sunrise F",
        )?;
    }
    let admission = Admission {
        case: name.into(),
        case_index,
        loops: scalar.loop_count(),
        projective_dimension: graph.powers().len() - 1,
        point: point.into(),
        powers: graph.powers().to_vec(),
        denominators: graph
            .family()
            .denominators()
            .iter()
            .map(ToString::to_string)
            .collect(),
        u: scalar.u().to_string(),
        f: scalar.f().to_string(),
        prefactor: scalar.prefactor().to_string(),
        u_exponent: scalar.u_exponent().to_string(),
        f_exponent: scalar.f_exponent().to_string(),
        measure_multiplier: graph.measure_multiplier().to_string(),
        native_scalar_numerator_verified: true,
        normalization: normalization.into(),
    };
    Ok(Fixture {
        graph,
        input: scalar.to_integrand(epsilon())?,
        admission,
        reference,
    })
}

fn admit_kite(graph: &GraphIntegral, scalar: &ScalarParametricIntegral) -> Result<()> {
    if graph.diagram().loop_count() != 2 || graph.powers() != [1, 1, 1, 1, 1] {
        return Err("kite topology or powers changed".into());
    }
    let family = graph.family();
    let k = family.loop_momenta();
    let p = symbols::external_momentum().call(0);
    let dot = |q: &Atom| family.kinematics().scalar_product(q, q);
    let expected = [
        dot(&k[0])? - 1,
        dot(&(&k[0] + &p))? - 1,
        dot(&(&k[0] - &k[1]))? - 1,
        dot(&k[1])?,
        dot(&(&k[1] + &p))?,
    ];
    for (actual, expected) in family.denominators().iter().zip(expected) {
        equal(actual, &expected, "kite denominator")?;
    }
    let u = parse!(
        "contour_scalar::x2*contour_scalar::x4+contour_scalar::x2*contour_scalar::x3+contour_scalar::x1*contour_scalar::x4+contour_scalar::x1*contour_scalar::x3+contour_scalar::x1*contour_scalar::x2+contour_scalar::x0*contour_scalar::x4+contour_scalar::x0*contour_scalar::x3+contour_scalar::x0*contour_scalar::x2"
    );
    // Equivalent factored form of the upstream expanded F polynomial, in the
    // exact same ordered five denominators: masses*U minus the kinetic forest.
    let mass_sum = parse!("contour_scalar::x0+contour_scalar::x1+contour_scalar::x2");
    let forest = parse!(
        "contour_scalar::x2*contour_scalar::x3*contour_scalar::x4+contour_scalar::x1*contour_scalar::x3*contour_scalar::x4+contour_scalar::x1*contour_scalar::x2*contour_scalar::x3+contour_scalar::x0*contour_scalar::x3*contour_scalar::x4+contour_scalar::x0*contour_scalar::x2*contour_scalar::x4+contour_scalar::x0*contour_scalar::x1*contour_scalar::x4+contour_scalar::x0*contour_scalar::x1*contour_scalar::x3+contour_scalar::x0*contour_scalar::x1*contour_scalar::x2"
    );
    equal(scalar.u(), &u, "kite U")?;
    equal(
        scalar.f(),
        &(mass_sum * &u - Atom::num((3, 1000)) * forest),
        "kite F",
    )?;
    equal(
        scalar.prefactor(),
        &parse!("gamma(1+2*contour_scalar::eps)"),
        "kite normalized prefactor",
    )?;
    equal(
        scalar.u_exponent(),
        &parse!("-1+3*contour_scalar::eps"),
        "kite U exponent",
    )?;
    equal(
        scalar.f_exponent(),
        &parse!("-1-2*contour_scalar::eps"),
        "kite F exponent",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_one_loop_inputs_use_independent_physical_masters() {
        for (name, dimension) in [("triangle", 2), ("box", 3)] {
            let fixture = build(name).unwrap();
            assert_eq!(fixture.admission.loops, 1);
            assert_eq!(fixture.admission.projective_dimension, dimension);
            assert!(fixture.reference.coefficients[&0][1].abs() > 1e-6);
            assert_eq!(fixture.reference.coefficients[&-1], [0., 0.]);
            assert_eq!(fixture.reference.coefficients[&-2], [0., 0.]);
            println!(
                "{name} native reference: {:?}",
                fixture.reference.coefficients
            );
        }
    }

    #[test]
    fn scalar_sunrise_keeps_native_scale_uv_pole_and_causal_phase() {
        let fixture = build("sunrise").unwrap();
        assert_eq!(fixture.admission.loops, 2);
        assert_eq!(fixture.input.parameters().len(), 3);
        assert_eq!(fixture.graph.powers(), [1, 1, 1]);
        let pole = fixture.reference.coefficients[&-1];
        assert!((pole[0] + 0.25).abs() < 1e-14 && pole[1].abs() < 1e-14);
        assert!((fixture.reference.coefficients[&0][1] + std::f64::consts::PI / 2.).abs() < 1e-13);
        println!(
            "sunrise native reference: {:?}",
            fixture.reference.coefficients
        );
    }

    #[test]
    fn native_kite_denominators_density_and_convergent_reference_are_admitted() {
        let fixture = build("kite").unwrap();
        assert_eq!(fixture.admission.loops, 2);
        assert_eq!(fixture.admission.projective_dimension, 4);
        let value = fixture.reference.coefficients[&0];
        // The upstream printed number uses a finite eta, not the exact i0
        // limit required by the integral. Reproduce its regulator natively
        // without changing the benchmark reference or loosening its tolerance.
        let (s, f0, f1) = kite_series();
        let eta = Atom::num((1_i64, 1_000_000_000_000_000_i64));
        let finite_regulator = complex(&(f0 + &f1 * (-&s - Atom::i() * eta).log())).unwrap();
        assert!((finite_regulator.re - 4.403658192582334).abs() < 2e-14);
        assert!((finite_regulator.im - 1.5704037847169694).abs() < 2e-14);
        // |log(1+i eta/s)| <= 2 eta/s here; this conservative finite-regulator
        // displacement is distinct from the much smaller series tail bound.
        let regulator_bound = 2. * complex(&f1).unwrap().re.abs() * 1e-15 / 0.003;
        assert!((value[0] - finite_regulator.re).abs() < regulator_bound);
        assert!((value[1] - finite_regulator.im).abs() < regulator_bound);
        assert!((value[1] - finite_regulator.im).abs() > 1e-13);
        assert!(fixture.reference.absolute_truncation_bound.unwrap() < 1e-26);
        println!(
            "kite native reference: {:?}; truncation bound: {:?}",
            fixture.reference.coefficients, fixture.reference.absolute_truncation_bound
        );
        assert!(build("unknown").is_err());
        assert_eq!(
            fixture
                .reference
                .values_for_layout(
                    &[-3, -2, -1, 0, 0],
                    &[
                        CoefficientComponent::Real,
                        CoefficientComponent::Imag,
                        CoefficientComponent::Real,
                        CoefficientComponent::Real,
                        CoefficientComponent::Imag
                    ]
                )
                .unwrap(),
            vec![0., 0., 0., value[0], value[1]]
        );
    }
}

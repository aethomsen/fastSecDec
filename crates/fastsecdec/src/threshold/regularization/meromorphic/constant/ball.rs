//! Reuse native exact-coefficient IR and certified directed ball operations.
use super::*;
use symbolica::{
    domains::float::{ComplexBall, Float, FloatLike, Real, RealBall, SingleFloat},
    evaluate::{Instruction, OptimizationSettings},
    id::{Pattern, Replacement},
};

#[derive(Clone, Debug)]
pub struct RadicalWitness {
    expression: Atom,
    radicand: RealBall,
    value: RealBall,
}
impl RadicalWitness {
    pub fn expression(&self) -> &Atom {
        &self.expression
    }
    pub fn radicand(&self) -> &RealBall {
        &self.radicand
    }
    pub fn value(&self) -> &RealBall {
        &self.value
    }
}
#[derive(Clone, Debug)]
pub struct DenominatorWitness {
    original: Atom,
    value: ComplexBall,
}
impl DenominatorWitness {
    pub fn original(&self) -> &Atom {
        &self.original
    }
    pub fn value(&self) -> &ComplexBall {
        &self.value
    }
}
#[derive(Clone, Debug)]
pub struct ConstantCoefficient {
    original: Atom,
    enclosure: ComplexBall,
    radicals: Vec<RadicalWitness>,
    denominators: Vec<DenominatorWitness>,
    bits: u32,
}
impl ConstantCoefficient {
    pub fn original(&self) -> &Atom {
        &self.original
    }
    pub fn enclosure(&self) -> &ComplexBall {
        &self.enclosure
    }
    pub fn radicals(&self) -> &[RadicalWitness] {
        &self.radicals
    }
    pub fn denominators(&self) -> &[DenominatorWitness] {
        &self.denominators
    }
    pub fn precision_bits(&self) -> u32 {
        self.bits
    }
}

fn collect(
    atom: AtomView<'_>,
    limits: Limits,
    depth: usize,
    nodes: &mut usize,
    roots: &mut Vec<Atom>,
    denominators: &mut Vec<Atom>,
) -> Result<()> {
    *nodes = nodes
        .checked_add(1)
        .ok_or(Error::ResourceIncomplete("constant coefficient nodes"))?;
    if *nodes > limits.source_nodes || depth > limits.source_depth {
        return Err(Error::ResourceIncomplete(
            "constant coefficient depth/nodes",
        ));
    }
    match atom {
        AtomView::Num(_) => {
            Complex::<Rational>::try_from(atom)
                .map_err(|_| Error::Unsupported("constant coefficient is not exact Q(i)"))?;
        }
        AtomView::Var(v) if v.get_symbol() == Symbol::PI => {}
        AtomView::Var(_) => return Err(Error::Unsupported("constant coefficient has free role")),
        AtomView::Add(sum) => {
            for a in sum.iter() {
                collect(a, limits, depth + 1, nodes, roots, denominators)?;
            }
        }
        AtomView::Mul(product) => {
            for a in product.iter() {
                collect(a, limits, depth + 1, nodes, roots, denominators)?;
            }
        }
        AtomView::Pow(power) => {
            let q = Rational::try_from(power.get_exp())
                .map_err(|_| Error::Unsupported("nonexact constant power"))?;
            if q != (1, 2) {
                let k = q
                    .numerator_ref()
                    .to_i64()
                    .filter(|_| q.is_integer())
                    .ok_or(Error::Unsupported(
                        "constant power outside integer/real-sqrt class",
                    ))?;
                if k.unsigned_abs() > limits.integer_power as u64 {
                    return Err(Error::ResourceIncomplete("constant integer power"));
                }
            }
            collect(
                power.get_base(),
                limits,
                depth + 1,
                nodes,
                roots,
                denominators,
            )?;
            if q == (1, 2) {
                roots.push(atom.to_owned());
            } else if q < 0 {
                denominators.push(power.get_base().to_owned());
            }
        }
        _ => {
            return Err(Error::Unsupported(
                "function outside certified constant class",
            ));
        }
    }
    Ok(())
}

fn evaluate(
    atom: &Atom,
    replacements: &[(Atom, Atom)],
    roles: &[Atom],
    values: &[ComplexBall],
    bits: u32,
    limits: Limits,
) -> Result<ComplexBall> {
    let expression =
        atom.replace_multiple(replacements.iter().map(|(a, b)| {
            Replacement::new(Pattern::Literal(a.clone()), Pattern::Literal(b.clone()))
        }));
    let evaluator = Atom::evaluator_multiple(&[expression], roles)
        .optimization_settings(
            OptimizationSettings::default()
                .cores(1)
                .cpe_iterations(Some(0)),
        )
        .build()
        .map_err(|e| Error::Native(e.to_string()))?;
    let ir = evaluator.export_instructions();
    if !ir.constant_functions.is_empty() || !ir.sub_evaluators.is_empty() {
        return Err(Error::Unsupported(
            "rounded/function constant evaluator fallback",
        ));
    }
    if ir.instructions.len() > limits.source_nodes.saturating_mul(4) {
        return Err(Error::ResourceIncomplete(
            "constant evaluator instruction allowance",
        ));
    }
    for instruction in &ir.instructions {
        match instruction {
            Instruction::Add(..) | Instruction::Mul(..) | Instruction::Assign(..) => {}
            Instruction::Pow(_, _, k, _) if k.unsigned_abs() <= limits.integer_power as u64 => {}
            _ => {
                return Err(Error::Unsupported(
                    "instruction outside certified integer arithmetic",
                ));
            }
        }
    }
    let zero = Rational::zero();
    let mut evaluator =
        evaluator.map_coeff_with_prec(&|q| ComplexBall::from_rational_ball(q, &zero, bits), bits);
    let mut out = [ComplexBall::from_rational_ball(
        &Complex::new(zero.clone(), zero.clone()),
        &zero,
        bits,
    )];
    evaluator
        .try_evaluate(values, &mut out)
        .map_err(|e| Error::Native(e.to_string()))?;
    if !out[0].is_finite() {
        return Err(Error::ResourceIncomplete(
            "constant ball enclosure nonfinite",
        ));
    }
    Ok(out[0].clone())
}

pub(super) fn certify(
    original: Atom,
    occupied: &HashSet<Symbol>,
    limits: Limits,
    constants: ConstantCoefficientLimits,
) -> Result<ConstantCoefficient> {
    let bits = constants.precision_bits;
    let mut roots = Vec::new();
    let mut denominator_bodies = Vec::new();
    collect(
        original.as_view(),
        limits,
        0,
        &mut 0,
        &mut roots,
        &mut denominator_bodies,
    )?;
    let mut occupied = occupied.clone();
    let mut next = 0usize;
    let mut fresh = || -> Result<Atom> {
        for _ in 0..1024 {
            let role = Symbol::parse(format!("slot_{next}"), "fastsecdec_prefactor_constant")
                .map_err(Error::Native)?;
            next = next
                .checked_add(1)
                .ok_or(Error::ResourceIncomplete("constant role counter"))?;
            if role.is_exportable() && occupied.insert(role) {
                return Ok(Atom::var(role));
            }
        }
        Err(Error::ResourceIncomplete("fresh constant coefficient role"))
    };
    let pi = fresh()?;
    let mut replacements = vec![(Atom::var(Symbol::PI), pi.clone())];
    let mut roles = vec![pi];
    let zero = RealBall::from_rational_ball(&Rational::zero(), &Rational::zero(), bits);
    // Existing native pi enclosure: nearest centre with upward error radius.
    // Existing adopted sqrt brackets each endpoint by g and directed a/g.
    let mut values = vec![Complex::new(
        RealBall::exact(Float::new(bits)).pi(),
        zero.clone(),
    )];
    let mut radicals = Vec::with_capacity(roots.len());
    for root in roots {
        let AtomView::Pow(power) = root.as_view() else {
            unreachable!()
        };
        let argument = evaluate(
            &power.get_base().to_owned(),
            &replacements,
            &roles,
            &values,
            bits,
            limits,
        )?;
        if !argument.is_real() {
            return Err(if argument.im.contains(&argument.im.center.zero()) {
                Error::ResourceIncomplete("radicand exact reality not certified")
            } else {
                Error::Unsupported("complex radicand outside real-sqrt class")
            });
        }
        if argument.re.lower_bound() < argument.re.center.zero() {
            return Err(if argument.re.upper_bound() < argument.re.center.zero() {
                Error::Unsupported("negative real radicand")
            } else {
                Error::ResourceIncomplete("radicand nonnegativity inconclusive")
            });
        }
        let value = argument.re.sqrt();
        if !value.is_finite() {
            return Err(Error::ResourceIncomplete(
                "real square-root enclosure nonfinite",
            ));
        }
        let slot = fresh()?;
        replacements.push((root.clone(), slot.clone()));
        roles.push(slot);
        values.push(Complex::new(value.clone(), zero.clone()));
        radicals.push(RadicalWitness {
            expression: root,
            radicand: argument.re,
            value,
        });
    }
    let mut denominators = Vec::with_capacity(denominator_bodies.len());
    for original in denominator_bodies {
        let value = evaluate(&original, &replacements, &roles, &values, bits, limits)?;
        if value.contains_zero() {
            return Err(Error::ResourceIncomplete(
                "constant denominator nonzero inconclusive",
            ));
        }
        denominators.push(DenominatorWitness { original, value });
    }
    let enclosure = evaluate(&original, &replacements, &roles, &values, bits, limits)?;
    Ok(ConstantCoefficient {
        original,
        enclosure,
        radicals,
        denominators,
        bits,
    })
}

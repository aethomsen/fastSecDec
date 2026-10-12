//! Native selected number-field arithmetic and native closed-box sign replay.
use super::*;
use symbolica::{
    domains::{
        RealEmbedding, Ring,
        algebraic::{AlgebraicContext, AlgebraicEmbedding, AlgebraicExtension},
        rational::Q,
        rational_polynomial::RationalPolynomial,
    },
    id::{Pattern, Replacement},
    poly::{PolyVariable, polynomial::MultivariatePolynomial},
};
use symgcad::{
    algebra::Algebra,
    domain::{certify_nonzero, derive_domain, verify_certificate},
};
pub(super) type Polynomial = MultivariatePolynomial<AlgebraicExtension<Q>, u32>;
pub(super) type Ratio = RationalPolynomial<AlgebraicExtension<Q>, u32>;
pub(super) fn pull(a: &Atom, from: &[Symbol], to: &[Atom]) -> Atom {
    a.replace_multiple(from.iter().zip(to).map(|(s, v)| {
        Replacement::new(Pattern::Literal(Atom::var(*s)), Pattern::Literal(v.clone()))
    }))
}
pub(super) struct Field {
    context: AlgebraicContext,
    variables: Arc<Vec<PolyVariable>>,
    symbols: Vec<Symbol>,
    primitive: Symbol,
    interval: [Rational; 2],
}
impl Field {
    pub(super) fn context(&self) -> &AlgebraicContext {
        &self.context
    }
    pub(super) fn interval(&self) -> &[Rational; 2] {
        &self.interval
    }
    pub(super) fn new(
        constants: &[Atom],
        symbols: &[Symbol],
        primitive: Symbol,
        w: &Work<'_>,
    ) -> Result<Self> {
        let mut context = AlgebraicContext::new(AlgebraicExtension::trivial(Q));
        for a in constants {
            w.atom(a)?;
            context.extend(a.as_view()).map_err(native)?;
            if context.field().poly().degree(0) as usize > w.limits.max_field_degree {
                return Err(Error::ResourceIncomplete(
                    "constant number-field degree".into(),
                ));
            }
        }
        let interval = if context.is_trivial() {
            [Rational::zero(), Rational::one()]
        } else {
            let field = context.field();
            field.try_sign(&field.generator()).map_err(native)?;
            let mut root = match field.embedding() {
                AlgebraicEmbedding::Isolated(r) => (*r).clone(),
                AlgebraicEmbedding::Indexed(i) => field
                    .poly()
                    .to_univariate_from_univariate(0)
                    .root(i)
                    .ok_or_else(|| invalid("native primitive embedding"))?,
            };
            root = root.refined(&Rational::from((1, 1_000_000)));
            if !matches!(
                root.classify_location(),
                symbolica::poly::univariate::RootLocation::Real
                    | symbolica::poly::univariate::RootLocation::Zero
            ) {
                return Err(unsupported("constant primitive field is not real"));
            }
            [
                &root.enclosure().center().re - root.enclosure().radius(),
                &root.enclosure().center().re + root.enclosure().radius(),
            ]
        };
        Ok(Self {
            context,
            variables: Arc::new(symbols.iter().copied().map(PolyVariable::from).collect()),
            symbols: symbols.to_vec(),
            primitive,
            interval,
        })
    }
    fn bounded(&self, p: &Polynomial, w: &Work<'_>) -> Result<()> {
        w.check()?;
        if p.variables() != &self.variables
            || p.nterms() > w.limits.algebra.max_terms
            || (0..p.nvars()).any(|i| p.degree(i) as usize > w.limits.algebra.max_degree)
        {
            return Err(Error::ResourceIncomplete(
                "projective polynomial roles/degree/terms".into(),
            ));
        }
        for c in &p.coefficients {
            for q in &c.poly().coefficients {
                w.budget.rational(q)?;
            }
        }
        Ok(())
    }
    pub(super) fn ratio(&mut self, a: &Atom, w: &Work<'_>) -> Result<Ratio> {
        w.atom(a)?;
        let before = self.context.field().clone();
        let r = self
            .context
            .to_rational_polynomial::<u32>(a.as_view(), self.variables.clone())
            .map_err(native)?;
        if self.context.field() != &before {
            return Err(invalid(
                "unexpected algebraic coefficient outside retained selected field",
            ));
        }
        self.bounded(&r.numerator, w)?;
        self.bounded(&r.denominator, w)?;
        Ok(r)
    }
    pub(super) fn polynomial(&mut self, a: &Atom, w: &Work<'_>) -> Result<Polynomial> {
        let r = self.ratio(a, w)?;
        if !r.denominator.is_constant() {
            return Err(invalid("expected native polynomial after fan pullback"));
        }
        let (q, rem) = r.numerator.quot_rem(&r.denominator, false);
        if !rem.is_zero() {
            return Err(invalid("native constant denominator division"));
        }
        Ok(q)
    }
    pub(super) fn expression(&self, p: &Polynomial) -> Result<Atom> {
        p.to_expression_with_context(&self.context).map_err(native)
    }
    pub(super) fn support(p: &Polynomial) -> Result<fastsecdec_sectors::PolynomialSupport> {
        fastsecdec_sectors::PolynomialSupport::new(
            p.into_iter()
                .map(|m| m.exponents.iter().map(|e| i64::from(*e)).collect())
                .collect(),
        )
        .map_err(native)
    }
    pub(super) fn extract(&self, p: &Polynomial, w: &Work<'_>) -> Result<(Polynomial, Vec<u32>)> {
        self.bounded(p, w)?;
        if p.is_zero() {
            return Err(unsupported("zero geometric pullback"));
        }
        let powers = (0..p.nvars())
            .map(|i| p.degree_bounds(i).0)
            .collect::<Vec<_>>();
        let m = p.monomial(p.ring().one(), powers.clone());
        let (q, r) = p.quot_rem(&m, false);
        if !r.is_zero() {
            return Err(invalid("native monomial division remainder"));
        }
        Ok((q, powers))
    }
    fn primitive_expression(&self, p: &Polynomial) -> Atom {
        p.into_iter()
            .map(|m| {
                let mut c = m.coefficient.poly().clone();
                let old = c.get_vars_ref()[0].clone();
                c.rename_variable(&old, &PolyVariable::from(self.primitive));
                m.exponents
                    .iter()
                    .zip(&self.symbols)
                    .fold(c.to_expression(), |a, (e, s)| {
                        a * Atom::var(*s).pow(Atom::num(*e))
                    })
            })
            .sum()
    }
    pub(super) fn unit(&self, p: &Polynomial, w: &Work<'_>) -> Result<ClosedUnit> {
        self.bounded(p, w)?;
        let symbols = self
            .symbols
            .iter()
            .copied()
            .chain([self.primitive])
            .collect::<Vec<_>>();
        let names = (0..symbols.len())
            .map(|i| format!("projective_unit_{i}"))
            .collect::<Vec<_>>();
        let algebra = Algebra::new(&names).map_err(native)?;
        let aliases = algebra
            .variables
            .iter()
            .map(|v| v.to_atom())
            .collect::<Vec<_>>();
        let expression = self.primitive_expression(p);
        let pp = algebra
            .parse(&pull(&expression, &symbols, &aliases).to_canonical_string())
            .map_err(native)?;
        let mut constraints = Vec::new();
        for (i, [lo, hi]) in self
            .symbols
            .iter()
            .map(|_| [Rational::zero(), Rational::one()])
            .chain([self.interval.clone()])
            .enumerate()
        {
            let q = algebra.parse(&names[i]).map_err(native)?;
            constraints.push(&q - &q.constant(lo));
            constraints.push(&q.constant(hi) - &q);
        }
        let domain = derive_domain(&algebra, &constraints, &[]).map_err(native)?;
        for sign in [1i8, -1] {
            for k in [4, 8, 16, 32] {
                w.check()?;
                let margin = Rational::from((1, 1i64 << k));
                let target = &pp * &pp.constant(Rational::from(i64::from(sign)))
                    - pp.constant(margin.clone());
                if let Some(certificate) =
                    certify_nonzero(&algebra, &target, &domain, &w.limits.algebra.certificates)
                        .map_err(native)?
                    && certificate.sign == 1
                {
                    verify_certificate(&algebra, &target, &constraints, &[], &certificate)
                        .map_err(native)?;
                    return Ok(ClosedUnit {
                        expression: self.expression(p)?,
                        polynomial: pp,
                        constraints,
                        sign,
                        margin,
                        certificate,
                    });
                }
            }
        }
        Err(unsupported("native closed residual margin not obtained"))
    }
    pub(super) fn receipt(
        &self,
        origin: FactorOrigin,
        source: Atom,
        n: &Polynomial,
        d: &Polynomial,
        w: &Work<'_>,
    ) -> Result<FactorReceipt> {
        let (n, np) = self.extract(n, w)?;
        let (d, dp) = self.extract(d, w)?;
        let numerator = self.unit(&n, w)?;
        let denominator = self.unit(&d, w)?;
        Ok(FactorReceipt {
            origin,
            source,
            powers: np
                .iter()
                .zip(dp)
                .map(|(a, b)| i64::from(*a) - i64::from(b))
                .collect(),
            numerator,
            denominator,
        })
    }
}

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
pub(in crate::threshold::regularization) type Polynomial =
    MultivariatePolynomial<AlgebraicExtension<Q>, u32>;
pub(in crate::threshold::regularization) type Ratio =
    RationalPolynomial<AlgebraicExtension<Q>, u32>;
pub(in crate::threshold::regularization) fn pull(a: &Atom, from: &[Symbol], to: &[Atom]) -> Atom {
    a.replace_multiple(from.iter().zip(to).map(|(s, v)| {
        Replacement::new(Pattern::Literal(Atom::var(*s)), Pattern::Literal(v.clone()))
    }))
}
pub(in crate::threshold::regularization) struct Field {
    context: AlgebraicContext,
    variables: Arc<Vec<PolyVariable>>,
    symbols: Vec<Symbol>,
    primitive: Symbol,
    interval: [Rational; 2],
    bounds: Vec<[Rational; 2]>,
}
impl Field {
    pub(in crate::threshold::regularization) fn context(&self) -> &AlgebraicContext {
        &self.context
    }
    pub(in crate::threshold::regularization) fn interval(&self) -> &[Rational; 2] {
        &self.interval
    }
    pub(in crate::threshold::regularization) fn new(
        constants: &[Atom],
        symbols: &[Symbol],
        primitive: Symbol,
        w: &Work<'_>,
    ) -> Result<Self> {
        Self::new_with_bounds(
            constants,
            symbols,
            primitive,
            &vec![[Rational::zero(), Rational::one()]; symbols.len()],
            &Rational::from((1, 1_000_000)),
            w,
        )
    }
    pub(in crate::threshold::regularization) fn new_with_bounds(
        constants: &[Atom],
        symbols: &[Symbol],
        primitive: Symbol,
        bounds: &[[Rational; 2]],
        precision: &Rational,
        w: &Work<'_>,
    ) -> Result<Self> {
        if symbols.len() != bounds.len() || bounds.iter().any(|b| b[0] >= b[1]) || *precision <= 0 {
            return Err(invalid("native coefficient-box roles/bounds/precision"));
        }
        w.budget.rational(precision)?;
        for q in bounds.iter().flatten() {
            w.budget.rational(q)?;
        }
        if symbols.contains(&symbolica::transcendental::root_var()) {
            return Err(invalid(
                "chart role collides with native root polynomial variable",
            ));
        }
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
        let variables = Arc::new(
            symbols
                .iter()
                .copied()
                .map(PolyVariable::from)
                .collect::<Vec<_>>(),
        );
        if !context.is_trivial()
            && context
                .field()
                .poly()
                .get_vars_ref()
                .iter()
                .any(|variable| variables.contains(variable))
        {
            return Err(invalid("chart role collides with native field generator"));
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
            root = root.refined(precision);
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
            variables,
            symbols: symbols.to_vec(),
            primitive,
            interval,
            bounds: bounds.to_vec(),
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
    pub(in crate::threshold::regularization) fn ratio(
        &mut self,
        a: &Atom,
        w: &Work<'_>,
    ) -> Result<Ratio> {
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
    pub(in crate::threshold::regularization) fn polynomial(
        &mut self,
        a: &Atom,
        w: &Work<'_>,
    ) -> Result<Polynomial> {
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
    pub(in crate::threshold::regularization) fn expression(&self, p: &Polynomial) -> Result<Atom> {
        if p.variables() != &self.variables || p.ring() != self.context.field() {
            return Err(invalid(
                "polynomial uses a different selected coefficient field",
            ));
        }
        if self.context.is_trivial() {
            // The rational field has no selected primitive to preserve. Its
            // native coefficient converter also avoids treating cached zero as
            // a preferred generator and forming 0^0 for new rational terms.
            Ok(p.to_expression())
        } else {
            // Retain the native primitive and its selected embedding. Contextual
            // conversion may simplify each uncached coefficient into a new
            // number field, an unnecessary cost for exact map pullbacks.
            Ok(p.to_expression_with_coeff_map(|ring, coefficient, out| {
                *out = ring.element_to_atom(coefficient);
            }))
        }
    }
    pub(in crate::threshold::regularization) fn support(
        p: &Polynomial,
    ) -> Result<fastsecdec_sectors::PolynomialSupport> {
        Self::support_in(p, p.nvars())
    }
    pub(in crate::threshold::regularization) fn support_in(
        p: &Polynomial,
        axes: usize,
    ) -> Result<fastsecdec_sectors::PolynomialSupport> {
        if axes > p.nvars() {
            return Err(invalid("support integration-role prefix"));
        }
        fastsecdec_sectors::PolynomialSupport::new(
            p.into_iter()
                .map(|m| m.exponents[..axes].iter().map(|e| i64::from(*e)).collect())
                .collect(),
        )
        .map_err(native)
    }
    pub(in crate::threshold::regularization) fn extract(
        &self,
        p: &Polynomial,
        w: &Work<'_>,
    ) -> Result<(Polynomial, Vec<u32>)> {
        self.extract_in(p, p.nvars(), w)
    }
    pub(in crate::threshold::regularization) fn extract_in(
        &self,
        p: &Polynomial,
        axes: usize,
        w: &Work<'_>,
    ) -> Result<(Polynomial, Vec<u32>)> {
        self.bounded(p, w)?;
        if p.is_zero() {
            return Err(unsupported("zero geometric pullback"));
        }
        if axes > p.nvars() {
            return Err(invalid("valuation integration-role prefix"));
        }
        let powers = (0..axes).map(|i| p.degree_bounds(i).0).collect::<Vec<_>>();
        let full = powers
            .iter()
            .copied()
            .chain(std::iter::repeat_n(0, p.nvars() - axes))
            .collect();
        let m = p.monomial(p.ring().one(), full);
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
    pub(in crate::threshold::regularization) fn unit(
        &self,
        p: &Polynomial,
        w: &Work<'_>,
    ) -> Result<ClosedUnit> {
        self.unit_with_margins(p, &[4, 8, 16, 32], w)
    }
    pub(in crate::threshold::regularization) fn unit_with_margins(
        &self,
        p: &Polynomial,
        margin_bits: &[u32],
        w: &Work<'_>,
    ) -> Result<ClosedUnit> {
        self.bounded(p, w)?;
        if margin_bits.is_empty() || margin_bits.iter().any(|b| *b > 256) {
            return Err(invalid("closed-unit margin proposal allowance"));
        }
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
            .bounds
            .iter()
            .cloned()
            .chain([self.interval.clone()])
            .enumerate()
        {
            let q = algebra.parse(&names[i]).map_err(native)?;
            constraints.push(&q - &q.constant(lo));
            constraints.push(&q.constant(hi) - &q);
        }
        let domain = derive_domain(&algebra, &constraints, &[]).map_err(native)?;
        for sign in [1i8, -1] {
            for k in margin_bits {
                w.check()?;
                let margin = Rational::one()
                    / Rational::from(
                        symbolica::domains::integer::Integer::from(2).pow(u64::from(*k)),
                    );
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
    pub(in crate::threshold::regularization) fn receipt(
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

//! Bounded admission to the existing native rational polynomial operations.
use super::{Budget, Error, Result, native};
use std::collections::BTreeSet;
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    domains::rational::Rational,
    id::{Pattern, Replacement},
};
use symgcad::{
    algebra::{Algebra, Poly},
    domain::{Domain, DomainCertificate, derive_domain},
};

pub(super) fn pull(a: &Atom, variables: &[Symbol], values: &[Atom]) -> Atom {
    a.replace_multiple(variables.iter().zip(values).map(|(s, v)| {
        Replacement::new(Pattern::Literal(Atom::var(*s)), Pattern::Literal(v.clone()))
    }))
}
#[derive(Debug)]
pub(crate) struct Ring {
    pub native: Algebra,
    symbols: Vec<Symbol>,
    aliases: Vec<Symbol>,
}
impl Ring {
    pub fn new(symbols: Vec<Symbol>) -> Result<Self> {
        let names = (0..symbols.len())
            .map(|i| format!("n{i}"))
            .collect::<Vec<_>>();
        Self::with_names(symbols, &names)
    }
    /// Preserve the native frame of retained certificates; alias conversion and
    /// its exact role/preflight checks are otherwise identical to `new`.
    pub(crate) fn with_names(symbols: Vec<Symbol>, names: &[String]) -> Result<Self> {
        if symbols.len() != names.len()
            || symbols.iter().collect::<BTreeSet<_>>().len() != symbols.len()
            || names.iter().collect::<BTreeSet<_>>().len() != names.len()
        {
            return Err(Error::Invalid(
                "distinct complete polynomial roles required".into(),
            ));
        }
        let native = Algebra::new(names).map_err(super::native)?;
        let aliases = native
            .variables
            .iter()
            .map(|v| {
                v.to_atom()
                    .as_view()
                    .as_var_view()
                    .expect("variable")
                    .get_symbol()
            })
            .collect();
        Ok(Self {
            native,
            symbols,
            aliases,
        })
    }
    pub fn polynomial(&self, a: &Atom, b: &Budget) -> Result<Poly> {
        b.check()?;
        if !a
            .get_all_symbols(true)
            .is_subset(&self.symbols.iter().copied().collect())
        {
            return Err(Error::Invalid(
                "unbound polynomial symbol before alias conversion".into(),
            ));
        }
        use crate::threshold::regularization::meromorphic::{Limits, preflight_expression};
        preflight_expression(
            a,
            Limits {
                degree: u16::try_from(b.limits.max_degree).map_err(native)?,
                terms: b.limits.max_terms,
                source_nodes: b.limits.max_terms.saturating_mul(8),
                integer_power: b.limits.max_degree as i64,
                ..Default::default()
            },
        )
        .map_err(|e| {
            Error::ResourceIncomplete(format!("normalized polynomial preflight: {e:?}"))
        })?;
        let converted = pull(
            a,
            &self.symbols,
            &self
                .aliases
                .iter()
                .copied()
                .map(Atom::var)
                .collect::<Vec<_>>(),
        );
        let p = self
            .native
            .parse(&converted.to_canonical_string())
            .map_err(native)?;
        check(&p, b)?;
        Ok(p)
    }
    pub fn expression(&self, p: &Poly) -> Atom {
        pull(
            &p.to_expression(),
            &self.aliases,
            &self
                .symbols
                .iter()
                .copied()
                .map(Atom::var)
                .collect::<Vec<_>>(),
        )
    }
    pub fn box_domain(&self, bounds: &[[Rational; 2]], b: &Budget) -> Result<(Vec<Poly>, Domain)> {
        if bounds.len() != self.symbols.len() {
            return Err(Error::Invalid(
                "complete rectangular coordinate inventory".into(),
            ));
        }
        let mut constraints = Vec::with_capacity(2 * bounds.len());
        for (symbol, [lo, hi]) in self.symbols.iter().zip(bounds) {
            b.rational(lo)?;
            b.rational(hi)?;
            if lo >= hi {
                return Err(Error::Invalid("ordered exact box bounds".into()));
            }
            constraints.push(self.polynomial(&(Atom::var(*symbol) - Atom::num(lo.clone())), b)?);
            constraints.push(self.polynomial(&(Atom::num(hi.clone()) - Atom::var(*symbol)), b)?);
        }
        let domain = derive_domain(&self.native, &constraints, &[]).map_err(native)?;
        if domain.empty {
            return Err(Error::Invalid("empty certificate box".into()));
        }
        Ok((constraints, domain))
    }
    pub fn positive(
        &self,
        p: &Poly,
        c: &[Poly],
        d: &Domain,
        b: &Budget,
    ) -> Result<DomainCertificate> {
        super::super::regular::certificate::positive(&self.native, p, c, d, b)?
            .ok_or_else(|| Error::Unsupported("native strict sign certificate not obtained".into()))
    }
}
pub(super) fn check(p: &Poly, b: &Budget) -> Result<()> {
    b.check()?;
    if p.nterms() > b.limits.max_terms
        || (0..p.nvars()).any(|i| p.degree(i) as usize > b.limits.max_degree)
    {
        return Err(Error::ResourceIncomplete(
            "normalized polynomial degree/term allowance".into(),
        ));
    }
    for c in &p.coefficients {
        b.rational(c)?;
    }
    Ok(())
}
/// Native monomial extraction only. Positive coefficients and nonnegative
/// powers prove interior positivity on the already admitted positive slots.
pub(super) fn monomial(p: &Poly) -> Result<Vec<u32>> {
    if p.nterms() != 1 || p.coefficients[0] <= Rational::zero() {
        return Err(Error::Unsupported(
            "positive monomial chart required".into(),
        ));
    }
    Ok((0..p.nvars()).map(|i| p.degree(i)).collect())
}

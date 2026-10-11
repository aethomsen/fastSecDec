use super::{Error, RelativeFace, Result};
use crate::threshold::maps::{
    endpoint_transport::cover::IssuedEndpointChart,
    normalized::algebra::Ring,
    regular::{Budget, certificate::positive},
};
use symbolica::{
    atom::{Atom, Symbol},
    domains::rational::Rational,
};
use symgcad::{
    algebra::Poly,
    domain::{Domain, DomainCertificate, verify_certificate},
};

/// Exact native evidence for positive margin. A constant/zero-role face uses
/// native Rational comparison, never a dummy polynomial coordinate.
#[derive(Debug)]
pub enum MarginEvidence {
    Constant(Rational),
    Native(Box<DomainCertificate>),
}
#[derive(Debug)]
pub struct MarginProof {
    polynomial: Atom,
    pub(super) margin: Rational,
    pub(super) evidence: MarginEvidence,
}
impl MarginProof {
    pub fn polynomial(&self) -> &Atom {
        &self.polynomial
    }
    pub fn margin(&self) -> &Rational {
        &self.margin
    }
    pub fn evidence(&self) -> &MarginEvidence {
        &self.evidence
    }
    pub fn replay(&self, domain: &ProofBox, b: &Budget) -> Result<()> {
        b.check()?;
        let target = &self.polynomial - Atom::num(self.margin.clone());
        if self.margin <= 0 {
            return Err(Error::Invalid("positive retained margin required".into()));
        }
        match &self.evidence {
            MarginEvidence::Constant(value) => {
                let actual = Rational::try_from(target.as_view())
                    .map_err(|e| Error::Native(e.to_string()))?;
                if &actual != value || actual <= 0 {
                    return Err(Error::Invalid("constant margin replay differs".into()));
                }
            }
            MarginEvidence::Native(c) => {
                let (ring, constraints, _) = domain
                    .native
                    .as_ref()
                    .ok_or_else(|| Error::Invalid("native proof without free roles".into()))?;
                let p = ring.polynomial(&target, b)?;
                verify_certificate(&ring.native, &p, constraints, &[], c)
                    .map_err(|e| Error::Native(e.to_string()))?;
                if c.sign != 1 {
                    return Err(Error::Invalid(
                        "retained certificate is not positive".into(),
                    ));
                }
            }
        }
        b.check()
    }
}
/// Full remaining unit box and actual retained root brackets. Polynomial
/// continuity extends a positive-margin proof to every boundary point; the
/// issued chart's polynomial closed-cube map and actual section Arcs supply
/// the selected-branch inclusion. No root equation is replaced by this box.
#[derive(Debug)]
pub struct ProofBox {
    roles: Vec<Symbol>,
    bounds: Vec<[Rational; 2]>,
    native: Option<(Ring, Vec<Poly>, Domain)>,
}
impl ProofBox {
    pub fn roles(&self) -> &[Symbol] {
        &self.roles
    }
    pub fn bounds(&self) -> &[[Rational; 2]] {
        &self.bounds
    }
    pub(super) fn new(
        chart: &IssuedEndpointChart,
        face: &RelativeFace,
        b: &Budget,
    ) -> Result<Self> {
        let mut roles = Vec::new();
        let mut bounds = Vec::new();
        for u in chart.target_units() {
            if !face.axes().iter().any(|a| a.unit == *u) {
                roles.push(*u);
                bounds.push([Rational::zero(), Rational::one()]);
            }
        }
        for r in chart.transport().root_relations() {
            let s = r.section();
            chart.section_parameter_images(s)?;
            roles.push(s.root_symbol());
            bounds.push([s.bracket().lower.clone(), s.bracket().upper.clone()]);
        }
        Self::from_bounds(roles, bounds, b)
    }
    pub(super) fn from_bounds(
        roles: Vec<Symbol>,
        bounds: Vec<[Rational; 2]>,
        b: &Budget,
    ) -> Result<Self> {
        if roles.len() != bounds.len() {
            return Err(Error::Invalid("complete face proof box required".into()));
        }
        let native = if roles.is_empty() {
            None
        } else {
            let r = Ring::new(roles.clone())?;
            let (c, d) = r.box_domain(&bounds, b)?;
            Some((r, c, d))
        };
        Ok(Self {
            roles,
            bounds,
            native,
        })
    }
    pub(super) fn propose(
        &self,
        a: &Atom,
        fallback: &Rational,
        b: &Budget,
    ) -> Result<Option<MarginProof>> {
        b.check()?;
        if let Ok(v) = Rational::try_from(a.as_view()) {
            if v <= 0 {
                return Ok(None);
            }
            let margin = &v / &Rational::from(2);
            b.rational(&v)?;
            b.rational(&margin)?;
            let proof = MarginProof {
                polynomial: a.clone(),
                margin: margin.clone(),
                evidence: MarginEvidence::Constant(&v - &margin),
            };
            proof.replay(self, b)?;
            return Ok(Some(proof));
        }
        let (ring, constraints, domain) = self
            .native
            .as_ref()
            .ok_or_else(|| Error::Invalid("unbound symbol on zero-role face".into()))?;
        let p = ring.polynomial(a, b)?;
        let constant = p.get_constant();
        let margin = if constant > 0 {
            constant / Rational::from(2)
        } else {
            fallback.clone()
        };
        b.rational(&margin)?;
        let mut candidates = vec![margin];
        if candidates[0] > *fallback {
            candidates.push(fallback.clone());
        }
        for margin in candidates {
            let target = &p - &p.constant(margin.clone());
            if let Some(c) = positive(&ring.native, &target, constraints, domain, b)? {
                // positive() already replays exactly; no second native search.
                return Ok(Some(MarginProof {
                    polynomial: a.clone(),
                    margin,
                    evidence: MarginEvidence::Native(Box::new(c)),
                }));
            }
        }
        // A finite candidate miss is not a proof that no positive margin exists.
        Ok(None)
    }
}

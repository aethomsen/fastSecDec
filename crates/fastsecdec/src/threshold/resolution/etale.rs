use super::{Budget, Error, Guard, Ideal, Poly};
use super::{LocalizedAlgebra, localized::Result};
use std::{collections::BTreeSet, sync::Arc};
use symbolica::{atom::AtomCore, domains::atom::AtomField, tensors::matrix::Matrix};

pub struct EtaleCertificate {
    pub source: Arc<LocalizedAlgebra>,
    /// Indices into source equations; unchosen equations must be redundant on
    /// this same localization. Parameters are not possible dependent axes.
    pub equations: Vec<usize>,
    pub dependent_axes: Vec<usize>,
    pub free_axes: Vec<usize>,
    /// None for the ordinary affine base case (empty selected system).
    pub determinant_inverse_axis: Option<usize>,
}
#[derive(Clone, Debug)]
pub struct EtaleFrame {
    source: Arc<LocalizedAlgebra>,
    local: Arc<LocalizedAlgebra>,
    equations: Vec<usize>,
    dependent_axes: Vec<usize>,
    free_axes: Vec<usize>,
    determinant: Poly,
    derivations: Vec<Vec<Poly>>,
}

/// Native chain rule with bounded checker-owned products. The native matrix
/// and Groebner internals still require caller-owned hard resource caps.
fn apply(coefficients: &[Poly], p: &Poly, budget: &mut Budget) -> Result<Poly> {
    budget.poly(p)?;
    let mut result = p.zero();
    for (axis, coefficient) in coefficients.iter().enumerate() {
        if coefficient.is_zero() {
            continue;
        }
        let term = budget.mul(coefficient, &p.derivative(axis))?;
        let bound = result
            .nterms()
            .checked_add(term.nterms())
            .ok_or(Error::ResourceIncomplete("derivation sum bound"))?;
        if bound > budget.limits.max_terms {
            return Err(Error::ResourceIncomplete("derivation sum bound"));
        }
        result = result + term;
    }
    budget.poly(&result)?;
    Ok(result)
}

fn close_derivation(
    local: &LocalizedAlgebra,
    coefficients: &mut [Poly],
    budget: &mut Budget,
    membership: &mut super::prepared_membership::PreparedLocalZero<'_>,
) -> Result<()> {
    let ring = local.ring();
    if !membership.is_for(local) {
        return Err(Error::Invalid("derivation membership localization owner"));
    }
    // Guard factors have no inverse-variable dependence by constructor.
    for guard in local.guards() {
        let derivative = apply(coefficients, &guard.factor, budget)?;
        let square = budget.power(&ring.coordinate(guard.inverse_axis)?, 2)?;
        coefficients[guard.inverse_axis] = -budget.mul(&square, &derivative)?;
    }
    for equation in local
        .ideal()
        .generators()
        .iter()
        .chain(local.unit_relations())
    {
        if !membership.zero(&apply(coefficients, equation, budget)?, budget)? {
            return Err(Error::Invalid(
                "relative derivative does not preserve relation",
            ));
        }
    }
    for (axis, coefficient) in coefficients.iter().enumerate() {
        if ring.is_parameter(axis) && !coefficient.is_zero() {
            return Err(Error::Invalid("parameter derivative must vanish"));
        }
    }
    Ok(())
}
impl EtaleCertificate {
    pub fn verify(self, budget: &mut Budget) -> Result<EtaleFrame> {
        let ring = self.source.ring();
        budget.reserve_slots(self.source.ideal().generators().len())?;
        for equation in self.source.ideal().generators() {
            budget.poly(equation)?;
        }
        let guard_count = self
            .source
            .guards()
            .len()
            .checked_add(1)
            .ok_or(Error::ResourceIncomplete("etale guard count"))?;
        budget.reserve_slots(guard_count)?;
        for guard in self.source.guards() {
            budget.poly(&guard.factor)?;
        }
        let n = self.equations.len();
        if n != self.dependent_axes.len()
            || self
                .equations
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .len()
                != n
            || self
                .equations
                .iter()
                .any(|i| *i >= self.source.ideal().generators().len())
        {
            return Err(Error::Invalid("selected equation dimensions"));
        }
        let roles = self
            .free_axes
            .iter()
            .chain(&self.dependent_axes)
            .copied()
            .collect::<Vec<_>>();
        let unique: BTreeSet<_> = roles.iter().copied().collect();
        if unique.len() != roles.len()
            || unique != self.source.axes().iter().copied().collect::<BTreeSet<_>>()
        {
            return Err(Error::Invalid("etale coordinate roles"));
        }
        if n == 0 {
            if self.determinant_inverse_axis.is_some() {
                return Err(Error::Invalid("affine base frame needs no inverse minor"));
            }
            let zero = Ideal::new(ring.clone(), vec![], budget)?;
            for equation in self.source.ideal().generators() {
                if !zero.contains(equation, self.source.unit_relations(), budget)? {
                    return Err(Error::Invalid(
                        "affine frame has an undeclared algebraic constraint",
                    ));
                }
            }
            let count = self
                .free_axes
                .len()
                .checked_mul(ring.len())
                .ok_or(Error::ResourceIncomplete("affine frame slots"))?;
            budget.reserve_slots(count)?;
            let mut derivations = Vec::new();
            let mut membership =
                super::prepared_membership::PreparedLocalZero::new(&self.source, budget)?;
            for free in &self.free_axes {
                let mut coefficients = vec![ring.one().zero(); ring.len()];
                coefficients[*free] = ring.one();
                close_derivation(&self.source, &mut coefficients, budget, &mut membership)?;
                derivations.push(coefficients);
            }
            let frame = EtaleFrame {
                determinant: ring.one(),
                local: self.source.clone(),
                source: self.source.clone(),
                equations: self.equations,
                dependent_axes: self.dependent_axes,
                free_axes: self.free_axes,
                derivations,
            };
            frame.check_commutators(budget, &mut membership)?;
            return Ok(frame);
        }
        let inverse_axis = self
            .determinant_inverse_axis
            .ok_or(Error::Invalid("nonempty system needs an inverse minor"))?;
        if inverse_axis >= ring.len()
            || ring.is_parameter(inverse_axis)
            || unique.contains(&inverse_axis)
            || self
                .source
                .guards()
                .iter()
                .any(|g| g.inverse_axis == inverse_axis)
        {
            return Err(Error::Invalid("inverse minor role"));
        }
        let entries_count = n
            .checked_mul(n)
            .ok_or(Error::ResourceIncomplete("Jacobian dimensions"))?;
        budget.reserve_slots(entries_count)?;
        let rows = self
            .equations
            .iter()
            .map(|i| &self.source.ideal().generators()[*i])
            .collect::<Vec<_>>();
        // Count native derivative terms rather than treating every entry as
        // dense. The row-sum product bounds determinant expansion. Including
        // the largest free-coordinate RHS term count per row also bounds every
        // Cramer numerator. Native elimination intermediates still require
        // caller-owned hard limits; this is an output-expression preflight.
        let mut sparse_bound = Some(1usize);
        for row in &rows {
            let dependent_terms = self.dependent_axes.iter().try_fold(0usize, |sum, axis| {
                sum.checked_add(row.derivative(*axis).nterms())
            });
            if dependent_terms == Some(0) {
                return Err(Error::Invalid("singular selected relative minor"));
            }
            let rhs_terms = self
                .free_axes
                .iter()
                .map(|axis| row.derivative(*axis).nterms())
                .max()
                .unwrap_or(0);
            sparse_bound = sparse_bound.and_then(|bound| {
                dependent_terms
                    .and_then(|terms| terms.checked_add(rhs_terms))
                    .and_then(|terms| bound.checked_mul(terms))
            });
        }
        // Retain the previous dense bound too: either independently valid
        // bound can admit the operation; overflow of both refuses it.
        let max_terms = rows.iter().map(|p| p.nterms()).max().unwrap_or(1);
        let dense_bound = (1..=n).try_fold(1usize, |bound, k| {
            bound
                .checked_mul(k)
                .and_then(|value| value.checked_mul(max_terms))
        });
        let term_bound = sparse_bound
            .into_iter()
            .chain(dense_bound)
            .min()
            .ok_or(Error::ResourceIncomplete("Jacobian determinant bound"))?;
        if term_bound > budget.limits.max_terms {
            return Err(Error::ResourceIncomplete("Jacobian determinant bound"));
        }
        for axis in 0..ring.len() {
            let degree = rows
                .iter()
                .try_fold(0usize, |sum, p| {
                    sum.checked_add(usize::from(p.degree(axis)))
                })
                .ok_or(Error::ResourceIncomplete("Jacobian degree bound"))?;
            if degree > usize::from(budget.limits.max_degree_per_axis) {
                return Err(Error::ResourceIncomplete("Jacobian degree bound"));
            }
        }
        let field = AtomField {
            statistical_zero_test: false,
            cancel_check_on_division: true,
            ..AtomField::new()
        };
        let dim = u32::try_from(n).map_err(|_| Error::ResourceIncomplete("Jacobian dimensions"))?;
        budget.charge(entries_count)?;
        let jacobian = Matrix::from_linear(
            rows.iter()
                .flat_map(|p| {
                    self.dependent_axes
                        .iter()
                        .map(move |axis| p.derivative(*axis).to_expression())
                })
                .collect(),
            dim,
            dim,
            field.clone(),
        )
        .map_err(|_| Error::Invalid("native Jacobian matrix"))?;
        let determinant = ring.atom(
            &jacobian
                .det()
                .map_err(|_| Error::Invalid("native determinant"))?
                .expand(),
        )?;
        budget.poly(&determinant)?;
        if determinant.is_zero() {
            return Err(Error::Invalid("singular selected relative minor"));
        }
        let mut guards = self.source.guards().to_vec();
        guards.push(Guard {
            factor: determinant.clone(),
            inverse_axis,
        });
        let local = LocalizedAlgebra::new(
            self.source.ideal().clone(),
            self.source.axes().to_vec(),
            guards,
            budget,
        )?;
        budget.reserve_slots(n)?;
        let selected = Ideal::new(
            ring.clone(),
            rows.iter().map(|p| (*p).clone()).collect(),
            budget,
        )?;
        for equation in self.source.ideal().generators() {
            if !selected.contains(equation, local.unit_relations(), budget)? {
                return Err(Error::Invalid(
                    "extra relation is not redundant on selected open",
                ));
            }
        }
        let count = self
            .free_axes
            .len()
            .checked_mul(ring.len())
            .ok_or(Error::ResourceIncomplete("relative frame slots"))?;
        budget.reserve_slots(count)?;
        let inverse = ring.coordinate(inverse_axis)?;
        let mut derivations = Vec::new();
        let mut membership = super::prepared_membership::PreparedLocalZero::new(&local, budget)?;
        for free in &self.free_axes {
            budget.reserve_slots(n)?;
            let rhs = Matrix::from_linear(
                rows.iter()
                    .map(|p| -p.derivative(*free).to_expression())
                    .collect(),
                dim,
                1,
                field.clone(),
            )
            .map_err(|_| Error::Invalid("native relative rhs"))?;
            budget.charge(1)?;
            let solved = jacobian
                .solve(&rhs)
                .map_err(|_| Error::Invalid("native relative solve"))?;
            let mut coefficients = vec![ring.one().zero(); ring.len()];
            coefficients[*free] = ring.one();
            for (i, axis) in self.dependent_axes.iter().enumerate() {
                let cleared = (determinant.to_expression() * &solved[(i as u32, 0)])
                    .together()
                    .expand();
                let numerator = ring.atom(&cleared)?;
                coefficients[*axis] = budget.mul(&inverse, &numerator)?;
            }
            close_derivation(&local, &mut coefficients, budget, &mut membership)?;
            derivations.push(coefficients);
        }
        let frame = EtaleFrame {
            source: self.source,
            local: local.clone(),
            equations: self.equations,
            dependent_axes: self.dependent_axes,
            free_axes: self.free_axes,
            determinant,
            derivations,
        };
        frame.check_commutators(budget, &mut membership)?;
        Ok(frame)
    }
}
impl EtaleFrame {
    pub fn source(&self) -> &Arc<LocalizedAlgebra> {
        &self.source
    }
    pub fn local(&self) -> &Arc<LocalizedAlgebra> {
        &self.local
    }
    pub fn selected_equations(&self) -> &[usize] {
        &self.equations
    }
    pub fn dependent_axes(&self) -> &[usize] {
        &self.dependent_axes
    }
    pub fn free_axes(&self) -> &[usize] {
        &self.free_axes
    }
    pub fn determinant(&self) -> &Poly {
        &self.determinant
    }
    pub fn coefficients(&self, free_index: usize) -> Result<&[Poly]> {
        self.derivations
            .get(free_index)
            .map(Vec::as_slice)
            .ok_or(Error::Invalid("free derivative index"))
    }
    pub fn derivative(&self, free_index: usize, p: &Poly, budget: &mut Budget) -> Result<Poly> {
        self.local.supports(p)?;
        apply(self.coefficients(free_index)?, p, budget)
    }
    fn check_commutators(
        &self,
        budget: &mut Budget,
        membership: &mut super::prepared_membership::PreparedLocalZero<'_>,
    ) -> Result<()> {
        if !membership.is_for(&self.local) {
            return Err(Error::Invalid("commutator membership localization owner"));
        }
        for i in 0..self.derivations.len() {
            for j in i + 1..self.derivations.len() {
                for axis in 0..self.local.ring().len() {
                    let difference = self.derivative(i, &self.derivations[j][axis], budget)?
                        - self.derivative(j, &self.derivations[i][axis], budget)?;
                    if !membership.zero(&difference, budget)? {
                        return Err(Error::Invalid("relative lifts do not commute"));
                    }
                }
            }
        }
        Ok(())
    }
}

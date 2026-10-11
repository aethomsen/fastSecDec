use super::*;

impl OriginalPhysicalState {
    pub(super) fn transport(
        &self,
        chart: &Arc<RelativeRecursiveChart>,
        restriction: Option<&LocalizedCoverHistory>,
        exceptional: &AccumulatedExceptional,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<(Vec<OriginalGeneratorPullback>, Poly)> {
        let source = chart.geometry().center().history().ledger().frame().local();
        let restrict = |p: &Poly, b: &mut Budget| match restriction {
            Some(r) => r.open().extension().pull(p, b),
            None => Ok(p.clone()),
        };
        let pull = |p: &Poly, b: &mut Budget| chart.geometry().pull(&restrict(p, b)?, b);
        b.reserve_slots(self.generators.len())?;
        let mut originals = Vec::new();
        let source_ideal = source.ideal().sum(chart.parent().source().ideal(), b)?;
        let mut membership =
            super::super::super::super::prepared_membership::PreparedMembership::new(
                &source_ideal,
                source.unit_relations(),
                b,
            )?;
        for (i, g) in self.generators.iter().enumerate() {
            let quotient = restrict(g.current_quotient(), b)?;
            // A normalized marked source may use another basis and coefficients.
            // Native membership supplies admissibility; transform EACH retained
            // original quotient directly, preserving the original polynomial key.
            if !membership.contains(&quotient, b)? {
                return Err(Error::Invalid(
                    "physical original quotient/source association",
                ));
            }
            let marked = Arc::new(MarkedIdeal::new(
                Ideal::new(source.ring().clone(), vec![quotient], b)?,
                chart.parent().source().mark(),
                b,
            )?);
            let transformed = controlled_marked(
                chart.geometry().clone(),
                marked,
                &format!("{namespace}_key{i}"),
                b,
            )?;
            let [current] = transformed.generators() else {
                return Err(Error::ResourceIncomplete(
                    "physical zero original factor needs component authority",
                ));
            };
            originals.push(OriginalGeneratorPullback::checked(
                g.source().clone(),
                pull(g.total(), b)?,
                exceptional.product().clone(),
                current.quotient.clone(),
                chart.geometry().frame().local(),
                b,
            )?);
        }
        let old_jacobian = pull(&self.jacobian, b)?;
        let jacobian = b.mul(
            &old_jacobian,
            chart.geometry().original_coordinate_jacobian(),
        )?;
        Ok((originals, jacobian))
    }
}

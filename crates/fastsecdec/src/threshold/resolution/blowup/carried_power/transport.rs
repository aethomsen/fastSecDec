use super::*;
impl CarriedPowerState {
    /// Follow an already-issued physical chart. This does not select/glue its
    /// center or authorize omitting any other physical chart/complement.
    pub fn advance(
        self: &Arc<Self>,
        chart: Arc<RelativeRecursiveChart>,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<CarriedPowerAdvance> {
        self.check_chart(&chart)?;
        if self.completed() {
            return Ok(CarriedPowerAdvance::SourceCompleted(self.clone()));
        }
        match self.transform(chart.clone(), namespace, b) {
            Ok(t) => Ok(CarriedPowerAdvance::Transformed(t)),
            Err(Error::ResourceIncomplete(reason)) => Ok(CarriedPowerAdvance::Incomplete(
                Arc::new(PendingPowerTransition {
                    source: self.clone(),
                    chart,
                    namespace: namespace.into(),
                    reason,
                }),
            )),
            Err(e) => Err(e),
        }
    }
    fn check_chart(&self, chart: &Arc<RelativeRecursiveChart>) -> Result<()> {
        if !Arc::ptr_eq(chart.parent().source(), self.chart.parent().target())
            || !Arc::ptr_eq(chart.geometry().center().history(), self.chart.history())
            || !Arc::ptr_eq(self.history.ledger().frame(), self.open.frame())
            || !Arc::ptr_eq(self.source_cosupport.frame(), self.open.frame())
            || !Arc::ptr_eq(self.source_cosupport.original().source(), &self.original)
            || !Arc::ptr_eq(self.source_cosupport.effective().source(), &self.effective)
        {
            return Err(Error::Invalid("carried power next physical parent/history"));
        }
        Ok(())
    }
    fn transform(
        self: &Arc<Self>,
        chart: Arc<RelativeRecursiveChart>,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<Arc<CarriedPowerTransition>> {
        self.check_chart(&chart)?;
        if self.completed() {
            return Err(Error::Invalid(
                "completed power source cannot be transformed",
            ));
        }
        let depth = self
            .depth
            .checked_add(1)
            .ok_or(Error::ResourceIncomplete("carried power ancestry"))?;
        b.reserve_slots(depth)?;
        let embedding = SupportEmbedding::strict(self.support.clone(), self.open.clone(), b)?;
        let lower = embedding
            .extension()
            .ideal(chart.geometry().center().ideal(), b)?;
        let normals = chart
            .geometry()
            .center()
            .normals()
            .iter()
            .map(|p| embedding.extension().pull(p, b))
            .collect::<Result<Vec<_>>>()?;
        for source in [&self.original, &self.normalized, &self.effective] {
            verify_marked_center(embedding.frame(), source, &lower, b)?;
        }
        if let CarriedPowerCompanion::Active(proof) = &self.companion {
            if !Arc::ptr_eq(proof.frame(), embedding.frame()) {
                return Err(Error::Invalid("carried active companion frame"));
            }
            for source in [proof.original().source(), proof.effective().source()] {
                verify_marked_center(embedding.frame(), source, &lower, b)?;
            }
        }
        let support = transport_embedding(
            chart.geometry().clone(),
            embedding,
            &format!("{namespace}_support"),
            b,
        )?;
        let center = Arc::new(MarkedIdeal::new(lower, 1, b)?);
        let unit_source = Arc::new(MarkedIdeal::new(
            Ideal::new(
                self.original.ideal().ring().clone(),
                vec![self.unit.clone()],
                b,
            )?,
            1,
            b,
        )?);
        let mut states = Vec::new();
        b.reserve_slots(support.opens().len())?;
        for (i, open) in support.opens().iter().enumerate() {
            let original = controlled_supported(
                support.clone(),
                open.clone(),
                self.original.clone(),
                &format!("{namespace}_original{i}"),
                b,
            )?;
            let normalized = controlled_supported(
                support.clone(),
                open.clone(),
                self.normalized.clone(),
                &format!("{namespace}_normalized{i}"),
                b,
            )?;
            let effective = controlled_supported(
                support.clone(),
                open.clone(),
                self.effective.clone(),
                &format!("{namespace}_effective{i}"),
                b,
            )?;
            let unit_transform = transform_supported(
                support.clone(),
                open.clone(),
                unit_source.clone(),
                0,
                &format!("{namespace}_unit{i}"),
                b,
            )?;
            let unit = single(&unit_transform)?.quotient.clone();
            let normalizer = QuotientNormalizer::prepare(open.frame().local().clone(), b)?;
            if !normalizer.verify_candidate(original.target(), normalized.target(), b)? {
                return Err(Error::Invalid("carried original normalization transform"));
            }
            verify_transforms(&normalized, &effective, &unit_transform, self.power()?, b)?;
            let companion = match &self.companion {
                CarriedPowerCompanion::Completed(proof) => {
                    CarriedPowerCompanion::Completed(proof.clone())
                }
                CarriedPowerCompanion::Active(proof) => {
                    let gp = controlled_supported(
                        support.clone(),
                        open.clone(),
                        proof.original().source().clone(),
                        &format!("{namespace}_Gp{i}"),
                        b,
                    )?;
                    let gg = controlled_supported(
                        support.clone(),
                        open.clone(),
                        proof.effective().source().clone(),
                        &format!("{namespace}_Gg{i}"),
                        b,
                    )?;
                    verify_transforms(&gp, &gg, &unit_transform, self.power()?, b)?;
                    CarriedPowerCompanion::from_proof(PowerCosupportProof::prove(
                        open.frame().clone(),
                        gp.target().clone(),
                        gg.target().clone(),
                        b,
                    )?)
                }
            };
            let center_transform = controlled_supported(
                support.clone(),
                open.clone(),
                center.clone(),
                &format!("{namespace}_center{i}"),
                b,
            )?;
            let boundary = LowerBoundaryTransform::prepare(
                self.history.clone(),
                chart.clone(),
                support.clone(),
                center_transform,
                normals.clone(),
                &format!("{namespace}_boundary{i}"),
                b,
            )?;
            let history = self.history.advanced_lower_boundary(&boundary, b)?;
            let source_cosupport = PowerCosupportProof::prove(
                open.frame().clone(),
                original.target().clone(),
                effective.target().clone(),
                b,
            )?;
            states.push(Arc::new(Self {
                origin: PowerStateOrigin::Continued {
                    prior: self.clone(),
                    chart: chart.clone(),
                },
                initial: self.initial.clone(),
                chart: chart.clone(),
                support: support.clone(),
                open: open.clone(),
                history,
                original: original.target().clone(),
                normalized: normalized.target().clone(),
                effective: effective.target().clone(),
                unit,
                normalizer,
                companion,
                source_cosupport,
                depth,
            }));
        }
        Ok(Arc::new(CarriedPowerTransition {
            source: self.clone(),
            chart,
            support,
            states,
        }))
    }
}
fn single(t: &SupportedMarkedTransform) -> Result<&super::super::coefficient::SupportedGenerator> {
    match t.generators() {
        [g] => Ok(g),
        _ => Err(Error::ResourceIncomplete(
            "carried controlled principal generator",
        )),
    }
}
fn verify_transforms(
    p: &SupportedMarkedTransform,
    g: &SupportedMarkedTransform,
    u: &SupportedMarkedTransform,
    k: usize,
    b: &mut Budget,
) -> Result<()> {
    if !Arc::ptr_eq(p.open(), g.open())
        || !Arc::ptr_eq(p.open(), u.open())
        || !Arc::ptr_eq(p.support(), g.support())
        || !Arc::ptr_eq(p.support(), u.support())
        || u.exceptional_power() != 0
    {
        return Err(Error::Invalid("carried power same-map transform"));
    }
    check_identity(
        p.open().frame(),
        p.target(),
        g.target(),
        &single(u)?.quotient,
        k,
        b,
    )?;
    let a = single(p)?;
    let z = single(g)?;
    let unit = single(u)?;
    let powered = b.power(&z.total, k)?;
    let rhs = b.mul(&unit.total, &powered)?;
    if !p.open().frame().local().zero(&(&a.total - &rhs), b)? {
        return Err(Error::Invalid("carried power total pullback identity"));
    }
    Ok(())
}

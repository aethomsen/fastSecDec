use super::super::super::differential::relative_differential_ideal;
use super::super::super::prepared_membership::PreparedMembership;
use super::*;

/// Derivatives are formed on this original frame before any restriction. A
/// contact-only vanishing statement cannot replace ambient-center admission.
#[derive(Clone, Debug)]
pub struct CompanionAncestorOrder {
    frame: Arc<EtaleFrame>,
    source: Arc<MarkedIdeal>,
    layers: Vec<Ideal>,
    clearings: Vec<Vec<UnitClearing>>,
}
impl CompanionAncestorOrder {
    pub fn frame(&self) -> &Arc<EtaleFrame> {
        &self.frame
    }
    pub fn source(&self) -> &Arc<MarkedIdeal> {
        &self.source
    }
    pub fn layers(&self) -> &[Ideal] {
        &self.layers
    }
    pub fn clearings(&self) -> &[Vec<UnitClearing>] {
        &self.clearings
    }
}

/// One original outer-source admissible center. The retained lower center is
/// local; no global maximum or canonical choice follows from this ascent.
#[derive(Clone, Debug)]
pub struct CompanionDispatchedCenter {
    origin: Arc<CompanionDispatchOrigin>,
    lower: Arc<DispatchedContactCenter>,
    cover: Arc<PhysicalSupportCover>,
    localization: Arc<LocalizedCoverHistory>,
    matched: Arc<MatchedPhysicalSupport>,
    original: Arc<MarkedIdeal>,
    companion: Arc<MarkedIdeal>,
    ideal: Ideal,
    normals: Vec<Poly>,
    clearings: Vec<UnitClearing>,
    orders: Vec<CompanionAncestorOrder>,
}
impl CompanionDispatchedCenter {
    pub fn origin(&self) -> &Arc<CompanionDispatchOrigin> {
        &self.origin
    }
    pub fn lower(&self) -> &Arc<DispatchedContactCenter> {
        &self.lower
    }
    pub fn cover(&self) -> &Arc<PhysicalSupportCover> {
        &self.cover
    }
    pub fn localization(&self) -> &Arc<LocalizedCoverHistory> {
        &self.localization
    }
    pub fn matched(&self) -> &Arc<MatchedPhysicalSupport> {
        &self.matched
    }
    pub fn frame(&self) -> &Arc<EtaleFrame> {
        self.matched.ambient()
    }
    pub fn history(&self) -> &Arc<ResolutionHistory> {
        self.localization.history()
    }
    pub fn source(&self) -> &Arc<MarkedIdeal> {
        &self.original
    }
    pub fn companion(&self) -> &Arc<MarkedIdeal> {
        &self.companion
    }
    pub fn ideal(&self) -> &Ideal {
        &self.ideal
    }
    pub fn normals(&self) -> &[Poly] {
        &self.normals
    }
    pub fn clearings(&self) -> &[UnitClearing] {
        &self.clearings
    }
    pub fn orders(&self) -> &[CompanionAncestorOrder] {
        &self.orders
    }

    pub(super) fn prepare(
        origin: Arc<CompanionDispatchOrigin>,
        lower: Arc<DispatchedContactCenter>,
        cover: Arc<PhysicalSupportCover>,
        localization: Arc<LocalizedCoverHistory>,
        matched: Arc<MatchedPhysicalSupport>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let (actual_cover, actual_open, _) = matched
            .support()
            .physical_localization()
            .ok_or(Error::Invalid("outer dispatch matched support owner"))?;
        if !Arc::ptr_eq(lower.components().branch().owner().origin(), origin.child())
            || !Arc::ptr_eq(cover.embedding(), origin.embedding())
            || !Arc::ptr_eq(cover.source(), origin.companion())
            || !Arc::ptr_eq(cover.lower(), lower.components().cover().full_cover())
            || !Arc::ptr_eq(actual_cover, &cover)
            || !Arc::ptr_eq(actual_open, &localization)
            || !Arc::ptr_eq(matched.lower(), lower.localization())
            || !Arc::ptr_eq(matched.lower_map().source(), lower.frame())
            || !Arc::ptr_eq(matched.ambient(), localization.history().ledger().frame())
            || !Arc::ptr_eq(localization.open().source(), origin.history())
        {
            return Err(Error::Invalid(
                "outer dispatch original/lower center association",
            ));
        }
        let original = Arc::new(MarkedIdeal::new(
            localization
                .open()
                .extension()
                .ideal(origin.original().ideal(), b)?,
            origin.original().mark(),
            b,
        )?);
        let companion = Arc::new(MarkedIdeal::new(
            localization
                .open()
                .extension()
                .ideal(origin.companion().ideal(), b)?,
            origin.companion().mark(),
            b,
        )?);
        let super::super::matched_ascent::MatchedCenterGeometry {
            ideal,
            normals,
            clearings,
            child,
        } = super::super::matched_ascent::lift_matched_center(
            &matched,
            &original,
            lower.ideal(),
            lower.normals(),
            b,
        )?;
        let ambient = matched.ambient().local();
        let center = ambient.ideal().sum(&ideal, b)?;
        let down = matched.frame().local();
        let child_center = down.ideal().sum(&child, b)?;
        let mut membership = PreparedMembership::new(&center, ambient.unit_relations(), b)?;
        let mut down_membership = PreparedMembership::new(&child_center, down.unit_relations(), b)?;
        let q = origin.coefficient().contact();
        b.reserve_slots(3)?;
        let mut orders = Vec::new();
        for (frame, source, contact_layer) in [
            (
                origin.history().ledger().frame().clone(),
                origin.original().clone(),
                false,
            ),
            (
                origin.history().ledger().frame().clone(),
                origin.companion().clone(),
                false,
            ),
        ]
        .into_iter()
        .chain(
            origin
                .coefficient()
                .coefficient()
                .boundary_ideal()
                .map(|v| (q.contact().clone(), Arc::new(v.clone()), true)),
        ) {
            b.reserve_slots(
                source
                    .mark()
                    .checked_mul(2)
                    .ok_or(Error::ResourceIncomplete("outer order receipt count"))?,
            )?;
            let mut layer = source.ideal().clone();
            let mut layers = Vec::new();
            let mut all_clearings = Vec::new();
            for order in 0..source.mark() {
                b.reserve_slots(layer.generators().len())?;
                let mut receipts = Vec::new();
                for f in layer.generators() {
                    let (n, d, clearing, at) = if contact_layer {
                        let in_lower = lower.localization().open().extension().pull(f, b)?;
                        let at = matched.lower_map().pull(&in_lower, b)?;
                        let c = clear_units(frame.local(), f, b)?;
                        let base = origin.history().ledger().frame().local().ring();
                        if [&c.numerator, &c.denominator]
                            .iter()
                            .any(|p| (base.len()..p.nvars()).any(|i| p.degree(i) > 0))
                        {
                            return Err(Error::ResourceIncomplete(
                                "outer old-incidence clearing retains graph slot",
                            ));
                        }
                        let n = super::super::super::elimination::remap(&c.numerator, base, b)?;
                        let d = super::super::super::elimination::remap(&c.denominator, base, b)?;
                        (
                            localization.open().extension().pull(&n, b)?,
                            localization.open().extension().pull(&d, b)?,
                            Some(c),
                            at,
                        )
                    } else {
                        let n = localization.open().extension().pull(f, b)?;
                        let at = matched.extension().pull(&n, b)?;
                        (n, ambient.ring().one(), None, at)
                    };
                    if !down_membership.contains(&at, b)? {
                        return Err(Error::Invalid(
                            "outer original order misses restricted center",
                        ));
                    }
                    if !membership.contains(&n, b)? {
                        return Err(Error::Invalid("outer original order misses ambient center"));
                    }
                    // d is a unit in a neighborhood of the closed center,
                    // which suffices to transfer this order/membership proof.
                    // It is NOT installed as an inverse on the whole ambient
                    // chart; the normal-coordinate check above requires that
                    // stronger authority separately.
                    let den = Ideal::new(ambient.ring().clone(), vec![d], b)?;
                    if !center.sum(&den, b)?.contains(
                        &ambient.ring().one(),
                        ambient.unit_relations(),
                        b,
                    )? {
                        return Err(Error::ResourceIncomplete(
                            "outer original order denominator needs ambient cover",
                        ));
                    }
                    if let Some(c) = clearing {
                        receipts.push(c);
                    }
                }
                all_clearings.push(receipts);
                layers.push(layer.clone());
                if order + 1 < source.mark() {
                    layer = relative_differential_ideal(&frame, &layer, b)?;
                }
            }
            orders.push(CompanionAncestorOrder {
                frame,
                source,
                layers,
                clearings: all_clearings,
            });
        }
        Ok(Arc::new(Self {
            origin,
            lower,
            cover,
            localization,
            matched,
            original,
            companion,
            ideal,
            normals,
            clearings,
            orders,
        }))
    }
}

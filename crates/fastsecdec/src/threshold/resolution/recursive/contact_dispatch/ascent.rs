use super::super::super::differential::relative_differential_ideal;
use super::super::super::prepared_membership::{PreparedLocalZero, PreparedMembership};
use super::super::component_localization::pull_ideal;
use super::*;

/// Original ambient derivative layers, formed before restricting any contact.
/// The original source mark and its actual maximum remain distinct.
#[derive(Clone, Debug)]
pub struct DispatchAncestorOrder {
    frame: Arc<EtaleFrame>,
    presentation: Arc<RecursivePresentation>,
    maximum: usize,
    layers: Vec<Ideal>,
    clearings: Vec<Vec<UnitClearing>>,
}
impl DispatchAncestorOrder {
    pub fn frame(&self) -> &Arc<EtaleFrame> {
        &self.frame
    }
    pub fn presentation(&self) -> &Arc<RecursivePresentation> {
        &self.presentation
    }
    pub fn maximum(&self) -> usize {
        self.maximum
    }
    pub fn layers(&self) -> &[Ideal] {
        &self.layers
    }
    pub fn clearings(&self) -> &[Vec<UnitClearing>] {
        &self.clearings
    }
}

/// A local admissible center from an actual zero terminal component. This is
/// neither a chosen global maximum nor an alternative original source.
#[derive(Clone, Debug)]
pub struct DispatchedContactCenter {
    components: Arc<ContactTerminalComponents>,
    localization: Arc<LocalizedCoverHistory>,
    support: Arc<SupportEmbedding>,
    parent: Arc<MarkedIdeal>,
    child: Arc<MarkedIdeal>,
    ideal: Ideal,
    normals: Vec<Poly>,
    ancestors: Vec<DispatchAncestorOrder>,
}
impl DispatchedContactCenter {
    pub fn components(&self) -> &Arc<ContactTerminalComponents> {
        &self.components
    }
    pub fn localization(&self) -> &Arc<LocalizedCoverHistory> {
        &self.localization
    }
    pub fn support(&self) -> &Arc<SupportEmbedding> {
        &self.support
    }
    pub fn frame(&self) -> &Arc<EtaleFrame> {
        self.support.ambient()
    }
    pub fn history(&self) -> &Arc<ResolutionHistory> {
        self.localization.history()
    }
    pub fn source(&self) -> &Arc<MarkedIdeal> {
        &self.parent
    }
    pub fn child(&self) -> &Arc<MarkedIdeal> {
        &self.child
    }
    pub fn ideal(&self) -> &Ideal {
        &self.ideal
    }
    pub fn normals(&self) -> &[Poly] {
        &self.normals
    }
    pub fn ancestors(&self) -> &[DispatchAncestorOrder] {
        &self.ancestors
    }
    pub(super) fn prepare(
        components: Arc<ContactTerminalComponents>,
        localization: Arc<LocalizedCoverHistory>,
        support: Arc<SupportEmbedding>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let branch = components.branch();
        let origin = branch.owner().origin();
        let (cover, open, map) = support
            .physical_localization()
            .ok_or(Error::Invalid("dispatch support restriction owner"))?;
        if !Arc::ptr_eq(cover, components.cover())
            || !Arc::ptr_eq(open, &localization)
            || !Arc::ptr_eq(localization.open().source(), origin.auxiliary().history())
            || !Arc::ptr_eq(support.ambient(), localization.history().ledger().frame())
        {
            return Err(Error::Invalid(
                "dispatch center original history/open owner",
            ));
        }
        let child_source = branch.contact().differential_coefficient();
        let child_ideal = pull_ideal(map, child_source.ideal(), b)?;
        let child = Arc::new(MarkedIdeal::new(child_ideal, child_source.mark(), b)?);
        for f in child.ideal().generators() {
            if !support.frame().local().zero(f, b)? {
                return Err(Error::Invalid("dispatch center requires actual zero child"));
            }
        }
        let original = origin.auxiliary().source();
        let parent = Arc::new(MarkedIdeal::new(
            localization.open().extension().ideal(original.ideal(), b)?,
            original.mark(),
            b,
        )?);
        let (ideal, normals, _) = super::super::lift::lift_embedded_geometry(
            support.ambient(),
            &parent,
            support.frame(),
            support.extension(),
            support.equations().generators().to_vec(),
            child.ideal(),
            &[],
            b,
        )?;
        if normals.is_empty() {
            return Err(Error::Invalid("dispatch center positive codimension"));
        }
        let count = origin
            .levels()
            .len()
            .checked_add(1)
            .ok_or(Error::ResourceIncomplete("dispatch ancestor count"))?;
        b.reserve_slots(count)?;
        let mut ancestors = Vec::new();
        let local = support.ambient().local();
        let center = local.ideal().sum(&ideal, b)?;
        let mut membership = PreparedMembership::new(&center, local.unit_relations(), b)?;
        let mut support_zero = PreparedLocalZero::new(support.frame().local(), b)?;
        for index in 0..count {
            let (frame, presentation, contact) = if index < origin.levels().len() {
                let level = &origin.levels()[index];
                (&level.frame, level.presentation(), level.contact())
            } else {
                (
                    origin.cover().frame(),
                    origin.presentation(),
                    branch.contact(),
                )
            };
            presentation.verify(frame, presentation.original_source())?;
            let maximum =
                presentation.original_maximum_order(contact.source().algebraic_maximum_order())?;
            if maximum < presentation.original_source().mark() {
                return Err(Error::Invalid(
                    "dispatch ancestor maximum below original mark",
                ));
            }
            // Mark admission caps the derivative-layer loop before allocations.
            b.reserve_slots(presentation.original_source().ideal().generators().len())?;
            MarkedIdeal::new(presentation.original_source().ideal().clone(), maximum, b)?;
            b.reserve_slots(maximum.checked_mul(2).ok_or(Error::ResourceIncomplete(
                "dispatch derivative receipt slots",
            ))?)?;
            let mut layer = presentation.original_source().ideal().clone();
            let mut layers = Vec::new();
            let mut clearings = Vec::new();
            for order in 0..maximum {
                b.reserve_slots(
                    layer
                        .generators()
                        .len()
                        .checked_mul(2)
                        .ok_or(Error::ResourceIncomplete("dispatch derivative layer slots"))?,
                )?;
                let mut receipts = Vec::new();
                for f in layer.generators() {
                    let mut pulled = f.clone();
                    for q in origin.levels()[index.min(origin.levels().len())..]
                        .iter()
                        .map(|l| l.contact())
                        .chain(std::iter::once(branch.contact()))
                    {
                        pulled = q.extension().pull(&pulled, b)?;
                    }
                    let at = map.pull(&pulled, b)?;
                    if !support_zero.zero(&at, b)? {
                        return Err(Error::Invalid(
                            "dispatch original ancestor order on support",
                        ));
                    }
                    // Zero on a smaller localized contact does not by itself
                    // establish admissibility on its ambient closed center.
                    let clearing = clear_units(branch.contact().contact().local(), &pulled, b)?;
                    let root_ring = origin.auxiliary().frame().local().ring();
                    if [&clearing.numerator, &clearing.denominator]
                        .iter()
                        .any(|p| (root_ring.len()..p.nvars()).any(|i| p.degree(i) > 0))
                    {
                        return Err(Error::ResourceIncomplete(
                            "dispatch ancestor clearing retains contact graph slot",
                        ));
                    }
                    let shrink = |p: &Poly, b: &mut Budget| {
                        super::super::super::elimination::remap(p, root_ring, b)
                    };
                    let n = localization
                        .open()
                        .extension()
                        .pull(&shrink(&clearing.numerator, b)?, b)?;
                    let d = localization
                        .open()
                        .extension()
                        .pull(&shrink(&clearing.denominator, b)?, b)?;
                    if !membership.contains(&n, b)? {
                        return Err(Error::Invalid(
                            "dispatch ancestor order misses ambient center",
                        ));
                    }
                    let denominator = Ideal::new(local.ring().clone(), vec![d], b)?;
                    if !center.sum(&denominator, b)?.contains(
                        &local.ring().one(),
                        local.unit_relations(),
                        b,
                    )? {
                        return Err(Error::ResourceIncomplete(
                            "dispatch ancestor denominator requires further ambient cover",
                        ));
                    }
                    receipts.push(clearing);
                }
                clearings.push(receipts);
                layers.push(layer.clone());
                if order + 1 < maximum {
                    layer = relative_differential_ideal(frame, &layer, b)?;
                }
            }
            ancestors.push(DispatchAncestorOrder {
                frame: frame.clone(),
                presentation: presentation.clone(),
                maximum,
                layers,
                clearings,
            });
        }
        Ok(Arc::new(Self {
            components,
            localization,
            support,
            parent,
            child,
            ideal,
            normals,
            ancestors,
        }))
    }
}

use super::super::clear_units;
use super::super::differential::relative_differential_ideal;
use super::*;

pub(super) fn ascend(
    level: Arc<RecursiveLevel>,
    child: Arc<RecursiveCenter>,
    b: &mut Budget,
) -> Result<RecursiveCenter> {
    let q = &level.contact;
    level.presentation.verify(&level.frame, &level.source)?;
    if !Arc::ptr_eq(&level.frame, q.source().frame())
        || !Arc::ptr_eq(&level.source, level.normalization().source())
        || level.presentation.effective_source().ideal() != q.source().source().as_ref()
        || !Arc::ptr_eq(&child.frame, q.contact())
        || child.source.ideal() != q.differential_coefficient().ideal()
        || child.source.mark() != q.differential_coefficient().mark()
    {
        return Err(Error::Invalid("recursive child/contact owner"));
    }
    let (ideal, normals, clearings) = lift_geometry(q, &level.frame, &level.source, &child, b)?;
    b.reserve_slots(
        child
            .invariant
            .len()
            .checked_add(1)
            .ok_or(Error::ResourceIncomplete("recursive invariant count"))?,
    )?;
    let mut invariant = vec![level.ratio.clone()];
    invariant.extend(child.invariant.iter().cloned());
    Ok(RecursiveCenter {
        frame: level.frame.clone(),
        source: level.source.clone(),
        ideal,
        normals,
        invariant,
        lift: Some(LiftReceipt {
            level,
            child,
            clearings,
        }),
    })
}

// Wrappers verify their exact construction owners before calling this shared geometry.
pub(super) fn lift_geometry(
    q: &ContactQuotient,
    parent_frame: &Arc<EtaleFrame>,
    parent_source: &Arc<MarkedIdeal>,
    child: &RecursiveCenter,
    b: &mut Budget,
) -> Result<(Ideal, Vec<Poly>, Vec<UnitClearing>)> {
    let candidate = &q.source().candidates()[q.candidate_index()];
    let contact = clear_units(parent_frame.local(), &candidate.equation, b)?;
    lift_embedded_geometry(
        parent_frame,
        parent_source,
        q.contact(),
        q.extension(),
        vec![contact.numerator],
        child.ideal(),
        child.normals(),
        b,
    )
}
/// Shared exact ascent; constructors must bind their actual embedding and
/// child producer before calling. Support normals are the checked smooth
/// defining equations, not arbitrary equations from a presentation.
#[allow(clippy::too_many_arguments)]
pub(crate) fn lift_embedded_geometry(
    parent_frame: &Arc<EtaleFrame>,
    parent_source: &Arc<MarkedIdeal>,
    child_frame: &Arc<EtaleFrame>,
    extension: &RingExtension,
    mut normals: Vec<Poly>,
    child_ideal: &Ideal,
    child_normals: &[Poly],
    b: &mut Budget,
) -> Result<(Ideal, Vec<Poly>, Vec<UnitClearing>)> {
    let old = parent_frame.local();
    let ring = old.ring();
    if extension.source() != ring
        || extension.target() != child_frame.local().ring()
        || child_ideal.ring() != child_frame.local().ring()
    {
        return Err(Error::Invalid("embedded lift rings"));
    }
    b.reserve_slots(
        child_normals
            .len()
            .checked_add(normals.len())
            .ok_or(Error::ResourceIncomplete("recursive lifted normal count"))?,
    )?;
    let mut clearings = Vec::new();
    for f in child_normals {
        let c = clear_units(child_frame.local(), f, b)?;
        if (ring.len()..c.numerator.nvars())
            .any(|i| c.numerator.degree(i) > 0 || c.denominator.degree(i) > 0)
        {
            return Err(Error::Invalid("recursive lift retained an auxiliary slot"));
        }
        let shrink = |p: &Poly| {
            p.rearrange_with_growth(ring.one().variables())
                .map_err(|_| Error::Invalid("recursive native variable-map drop"))
        };
        let n = shrink(&c.numerator)?;
        let d = shrink(&c.denominator)?;
        let unit = old.ideal().sum(&Ideal::new(ring.clone(), vec![d], b)?, b)?;
        if !unit.contains(&ring.one(), old.unit_relations(), b)? {
            return Err(Error::Invalid("recursive clearing unit fails upstairs"));
        }
        normals.push(n);
        clearings.push(c);
    }
    let ideal = Ideal::new(ring.clone(), normals.clone(), b)?;
    if old
        .ideal()
        .sum(&ideal, b)?
        .contains(&ring.one(), old.unit_relations(), b)?
    {
        return Err(Error::Invalid("recursive lifted center empty"));
    }
    let pulled = extension.ideal(&ideal, b)?;
    let down = child_frame.local();
    for (a, c) in [(&pulled, child_ideal), (child_ideal, &pulled)] {
        let c = down.ideal().sum(c, b)?;
        for f in a.generators() {
            if !c.contains(f, down.unit_relations(), b)? {
                return Err(Error::Invalid("recursive lifted center restriction"));
            }
        }
    }
    // Admissibility in the actual parent source, using the shared full relative
    // derivative engine. No differentiation in parameter directions.
    let center = old.ideal().sum(&ideal, b)?;
    let mut layer = parent_source.ideal().clone();
    for j in 0..parent_source.mark() {
        for f in layer.generators() {
            if !center.contains(f, old.unit_relations(), b)? {
                return Err(Error::Invalid("recursive parent source order below mark"));
            }
        }
        if j + 1 < parent_source.mark() {
            layer = relative_differential_ideal(parent_frame, &layer, b)?;
        }
    }
    Ok((ideal, normals, clearings))
}

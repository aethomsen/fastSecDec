//! Strict embedded contact support, with native saturation and checked relative
//! frames on a complete minor cover. Empty support is a geometric outcome.
use super::super::*;
use super::{
    embedding::SupportEmbedding, general_transform::RelativeBlowupGeometry, helpers::*,
    saturation::SaturatedSupport, support_preparation::StrictSupportPreparation,
};
use std::sync::Arc;
use symbolica::symbol;

#[derive(Clone, Debug)]
pub struct StrictSupportOpen {
    extension: RingExtension,
    frame: Arc<EtaleFrame>,
    relative_minor: Poly,
    chosen_equations: Vec<usize>,
    columns: Vec<usize>,
}
impl StrictSupportOpen {
    pub fn extension(&self) -> &RingExtension {
        &self.extension
    }
    pub fn frame(&self) -> &Arc<EtaleFrame> {
        &self.frame
    }
    pub fn relative_minor(&self) -> &Poly {
        &self.relative_minor
    }
    pub fn chosen_equations(&self) -> &[usize] {
        &self.chosen_equations
    }
    pub fn columns(&self) -> &[usize] {
        &self.columns
    }
}
/// A nonempty attempted minor open whose selected regular equations do not
/// generate the complete localized strict-support ideal. This is not an empty
/// physical chart or a certificate of singularity. Only a separately verified
/// full cover by admitted frames permits the support producer to complete.
#[derive(Clone, Debug)]
pub struct UnresolvedSupportFrame {
    extension: RingExtension,
    chosen_equations: Vec<usize>,
    columns: Vec<usize>,
    relations: Ideal,
    selected: Arc<LocalizedAlgebra>,
    missing_relation: Poly,
}
impl UnresolvedSupportFrame {
    pub fn extension(&self) -> &RingExtension {
        &self.extension
    }
    pub fn chosen_equations(&self) -> &[usize] {
        &self.chosen_equations
    }
    pub fn columns(&self) -> &[usize] {
        &self.columns
    }
    pub fn relations(&self) -> &Ideal {
        &self.relations
    }
    pub fn selected(&self) -> &Arc<LocalizedAlgebra> {
        &self.selected
    }
    pub fn missing_relation(&self) -> &Poly {
        &self.missing_relation
    }
}
#[derive(Clone, Debug)]
pub struct StrictContactSupport {
    preparation: Arc<StrictSupportPreparation>,
    geometry: Arc<RelativeBlowupGeometry>,
    embedding: Arc<SupportEmbedding>,
    saturation: Arc<SaturatedSupport>,
    clearings: Vec<UnitClearing>,
    source_units: Vec<Poly>,
    opens: Vec<Arc<StrictSupportOpen>>,
    empty_opens: Vec<EmptyAdaptedOpen>,
    unresolved_frames: Vec<UnresolvedSupportFrame>,
    cover: VerifiedOpenCover,
}
impl StrictContactSupport {
    pub fn preparation(&self) -> &Arc<StrictSupportPreparation> {
        &self.preparation
    }
    pub fn geometry(&self) -> &Arc<RelativeBlowupGeometry> {
        &self.geometry
    }
    pub fn source(&self) -> &Arc<ContactQuotient> {
        self.embedding.original_contact()
    }
    pub fn embedding(&self) -> &Arc<SupportEmbedding> {
        &self.embedding
    }
    pub fn saturation(&self) -> &Arc<SaturatedSupport> {
        &self.saturation
    }
    pub fn clearings(&self) -> &[UnitClearing] {
        &self.clearings
    }
    pub fn source_units(&self) -> &[Poly] {
        &self.source_units
    }
    pub fn opens(&self) -> &[Arc<StrictSupportOpen>] {
        &self.opens
    }
    pub fn empty_opens(&self) -> &[EmptyAdaptedOpen] {
        &self.empty_opens
    }
    pub fn unresolved_frames(&self) -> &[UnresolvedSupportFrame] {
        &self.unresolved_frames
    }
    pub fn cover(&self) -> &VerifiedOpenCover {
        &self.cover
    }
}
pub fn transport_contact_support(
    geometry: Arc<RelativeBlowupGeometry>,
    source: Arc<ContactQuotient>,
    namespace: &str,
    b: &mut Budget,
) -> Result<Arc<StrictContactSupport>> {
    let embedding = SupportEmbedding::contact(source, b)?;
    transport_embedding(geometry, embedding, namespace, b)
}
pub(crate) fn transport_embedding(
    geometry: Arc<RelativeBlowupGeometry>,
    embedding: Arc<SupportEmbedding>,
    namespace: &str,
    b: &mut Budget,
) -> Result<Arc<StrictContactSupport>> {
    let preparation =
        StrictSupportPreparation::prepare(geometry.clone(), embedding.clone(), namespace, b)?;
    let target = geometry.frame();
    let local = target.local();
    let saturation = preparation.saturation().clone();
    let source_units = preparation.ambient_units(b)?;
    let codim = embedding.codimension();
    if codim == 0 || codim > target.free_axes().len() {
        return Err(Error::Invalid("embedded support relative codimension"));
    }
    let clearings = saturation
        .result()
        .generators()
        .iter()
        .map(|f| clear_units(local, f, b))
        .collect::<Result<Vec<_>>>()?;
    let mut opens = Vec::new();
    let mut empty_opens = Vec::new();
    let mut unresolved_frames = Vec::new();
    if !saturation.empty() {
        for chosen in combinations(clearings.len(), codim, b)? {
            for columns in combinations(target.free_axes().len(), codim, b)? {
                let ns = format!(
                    "{namespace}_eq{}_col{}",
                    chosen
                        .iter()
                        .map(usize::to_string)
                        .collect::<Vec<_>>()
                        .join("_"),
                    columns
                        .iter()
                        .map(usize::to_string)
                        .collect::<Vec<_>>()
                        .join("_")
                );
                let extension = RingExtension::new(
                    local.ring().clone(),
                    [
                        symbol!(format!("{ns}::inverse")),
                        symbol!(format!("{ns}::spare")),
                    ],
                    b,
                )?;
                let ring = extension.target();
                let inverse = extension.source().len();
                let mut entries = Vec::new();
                for i in &chosen {
                    for j in &columns {
                        entries.push(target.derivative(*j, &clearings[*i].numerator, b)?);
                    }
                }
                let relative_minor = determinant(local.ring(), entries, codim, b)?;
                let mut equations = extension.ideal(local.ideal(), b)?.generators().to_vec();
                equations.extend(
                    clearings
                        .iter()
                        .map(|c| extension.pull(&c.numerator, b))
                        .collect::<Result<Vec<_>>>()?,
                );
                let relations = Ideal::new(ring.clone(), equations, b)?;
                let guards = extension.guards(local.guards(), b)?;
                let mut probe_guards = guards.clone();
                let minor_clear = clear_units(local, &relative_minor, b)?;
                probe_guards.push(Guard {
                    factor: extension.pull(&minor_clear.numerator, b)?,
                    inverse_axis: inverse,
                });
                if empty(&relations, &probe_guards, b)? {
                    empty_opens.push(EmptyAdaptedOpen {
                        incidence: chosen.clone(),
                        columns: columns.clone(),
                        ideal: relations,
                        guards: probe_guards,
                    });
                    continue;
                }
                let mut selected = target
                    .selected_equations()
                    .iter()
                    .map(|i| extension.pull(&target.source().ideal().generators()[*i], b))
                    .collect::<Result<Vec<_>>>()?;
                selected.extend(
                    chosen
                        .iter()
                        .map(|i| extension.pull(&clearings[*i].numerator, b))
                        .collect::<Result<Vec<_>>>()?,
                );
                // A nonzero minor is insufficient: selected equations may
                // retain extra components away from the actual support. Check
                // the full localized ideal before admitting any Etale frame.
                let selected_local = LocalizedAlgebra::new(
                    Ideal::new(ring.clone(), selected.clone(), b)?,
                    local.axes().to_vec(),
                    probe_guards.clone(),
                    b,
                )?;
                let mut missing = None;
                for f in relations.generators() {
                    if !selected_local.zero(f, b)? {
                        missing = Some(f.clone());
                        break;
                    }
                }
                if let Some(missing_relation) = missing {
                    b.reserve_slots(1)?;
                    unresolved_frames.push(UnresolvedSupportFrame {
                        extension,
                        chosen_equations: chosen.clone(),
                        columns,
                        relations,
                        selected: selected_local,
                        missing_relation,
                    });
                    continue;
                }
                let mut dependent = target.dependent_axes().to_vec();
                dependent.extend(columns.iter().map(|j| target.free_axes()[*j]));
                let free = target
                    .free_axes()
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| !columns.contains(j))
                    .map(|(_, a)| *a)
                    .collect();
                let restricted =
                    LocalizedAlgebra::new(relations.clone(), local.axes().to_vec(), guards, b)?;
                let frame = Arc::new(
                    EtaleCertificate {
                        source: restricted,
                        equations: indices(&relations, &selected)?,
                        dependent_axes: dependent,
                        free_axes: free,
                        determinant_inverse_axis: Some(inverse),
                    }
                    .verify(b)?,
                );
                let expected = b.mul(target.determinant(), &relative_minor)?;
                if !frame
                    .local()
                    .zero(&(frame.determinant() - &extension.pull(&expected, b)?), b)?
                {
                    return Err(Error::Invalid("strict support relative minor identity"));
                }
                b.reserve_slots(1)?;
                opens.push(Arc::new(StrictSupportOpen {
                    extension,
                    frame,
                    relative_minor,
                    chosen_equations: chosen.clone(),
                    columns,
                }));
            }
        }
    }
    let cover = OpenCoverCertificate {
        algebra: local.clone(),
        support: (**saturation.result()).clone(),
        opens: opens.iter().map(|o| o.relative_minor.clone()).collect(),
    }
    .verify(b)?;
    Ok(Arc::new(StrictContactSupport {
        preparation,
        geometry,
        embedding,
        saturation,
        clearings,
        source_units,
        opens,
        empty_opens,
        unresolved_frames,
        cover,
    }))
}

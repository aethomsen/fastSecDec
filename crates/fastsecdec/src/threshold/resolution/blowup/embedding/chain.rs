//! Pull through only receipt-issued same-support restrictions/refinements.
use super::*;
impl SupportEmbedding {
    pub(crate) fn pull_refinement(
        self: &Arc<Self>,
        root: &Arc<Self>,
        value: &Poly,
        ambient: bool,
        b: &mut Budget,
    ) -> Result<Poly> {
        b.charge(1)?;
        if Arc::ptr_eq(self, root) {
            let local = if ambient {
                self.ambient().local()
            } else {
                self.frame().local()
            };
            local.supports(value)?;
            b.poly(value)?;
            return Ok(value.clone());
        }
        match &self.origin {
            EmbeddingOrigin::SourceOpen {
                parent,
                localization,
                map,
            }
            | EmbeddingOrigin::AmbientOpen {
                parent,
                localization,
                map,
                ..
            } => {
                let p = parent.pull_refinement(root, value, ambient, b)?;
                if ambient {
                    localization.open().extension().pull(&p, b)
                } else {
                    map.pull(&p, b)
                }
            }
            EmbeddingOrigin::Matched { parent, matched } => {
                let p = parent.pull_refinement(root, value, ambient, b)?;
                if ambient {
                    Ok(p)
                } else {
                    matched.support_map().pull(&p, b)
                }
            }
            _ => Err(Error::Invalid(
                "refinement cannot replace original support edge",
            )),
        }
    }
}

//! Opt-in interpretation of native local fragments, never a graph reconstruction.
use super::*;
use idenso::tensor::SymbolicTensor;

/// Numerical input convention for graph payloads. This changes the meaning of
/// a rounded aggregate: each stored local fragment is exactified first, then
/// the native graph recomputes their product. The original aggregate is retained
/// as provenance, not asserted equal to that product or multiplied again.
///
/// Currently only Float coefficients outside function arguments are admitted.
/// This deliberately refuses legitimate scalar `f(0.1)` and shorthand arguments
/// as well as tensor-index/momentum labels. It is not full ggHH payload support.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphPayloadPolicy {
    PrimitiveFragmentsV1,
}

fn atom(work: &mut Work<GraphLocation>, original: &Atom, location: GraphLocation) -> Result<Atom> {
    work.atom_checked(original, location, true, || {
        // Native index/interface admission on the ORIGINAL expression. The
        // bounded role fence runs first; inference alone does not distinguish
        // a Float momentum label from a numerical scalar argument.
        SymbolicTensor::infer(original.clone())
            .map(|_| ())
            .map_err(|e| Error::Invalid(format!("original graph payload: {e}")))
    })
}

pub(super) fn convert(
    original: &FeynmanDiagram,
    work: &mut Work<GraphLocation>,
    mut observe: impl FnMut(&Work<GraphLocation>) -> Result<()>,
) -> Result<FeynmanDiagram> {
    // Observe stored aggregate independently; this value never defines the
    // replacement density. All original precision remains in the witness.
    let _ = atom(
        work,
        original.numerator(),
        GraphLocation::OriginalAggregate {},
    )?;
    let mut vertices = Vec::new();
    for (id, vertex) in original.vertices() {
        observe(work)?;
        vertices.push((
            id,
            atom(
                work,
                &vertex.numerator,
                GraphLocation::Payload {
                    role: format!("vertex_numerator:{}", id.0),
                },
            )?,
        ));
    }
    let mut edges = Vec::new();
    for (id, _, edge) in original.edges() {
        observe(work)?;
        edges.push((
            id,
            atom(
                work,
                &edge.numerator,
                GraphLocation::Payload {
                    role: format!("edge_numerator:{}", id.0),
                },
            )?,
        ));
    }
    let mut global = |expression, role: &str| {
        observe(work)?;
        atom(
            work,
            expression,
            GraphLocation::Payload { role: role.into() },
        )
    };
    let prefactor = global(original.numerator_prefactor(), "numerator_prefactor")?;
    let projector = global(original.projector(), "projector")?;
    let overall = global(original.overall_factor(), "overall_factor")?;
    // Every source payload was bounded/admitted before native validation or
    // graph arithmetic. Original and exact diagrams remain distinct owners.
    original
        .validate()
        .map_err(|e| Error::Invalid(e.to_string()))?;
    let exact = original
        .clone()
        .with_numerator_fragments(vertices, edges)
        .map_err(|e| Error::Invalid(e.to_string()))?
        .with_numerator_prefactor(prefactor)
        .with_projector(projector)
        .with_overall_factor(overall);
    exact
        .validate()
        .map_err(|e| Error::Invalid(e.to_string()))?;
    Ok(exact)
}

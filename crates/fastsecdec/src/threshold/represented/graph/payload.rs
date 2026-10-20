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
        original.numerator().expression(),
        GraphLocation::OriginalAggregate {},
    )?;
    let mut vertices = Vec::new();
    for (id, vertex) in original.vertices() {
        observe(work)?;
        vertices.push((
            id,
            vertex
                .numerator
                .with_checked_expression(atom(
                    work,
                    vertex.numerator.expression(),
                    GraphLocation::Payload {
                        role: format!("vertex_numerator:{}", id.0),
                    },
                )?)
                .map_err(|e| Error::Invalid(e.to_string()))?,
        ));
    }
    let mut edges = Vec::new();
    for (id, _, edge) in original.edges() {
        observe(work)?;
        edges.push((
            id,
            edge.numerator
                .with_checked_expression(atom(
                    work,
                    edge.numerator.expression(),
                    GraphLocation::Payload {
                        role: format!("edge_numerator:{}", id.0),
                    },
                )?)
                .map_err(|e| Error::Invalid(e.to_string()))?,
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
    let prefactor = original
        .numerator_prefactor()
        .with_checked_expression(global(
            original.numerator_prefactor().expression(),
            "numerator_prefactor",
        )?)
        .map_err(|e| Error::Invalid(e.to_string()))?;
    let projector = original
        .projector()
        .with_checked_expression(global(original.projector().expression(), "projector")?)
        .map_err(|e| Error::Invalid(e.to_string()))?;
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

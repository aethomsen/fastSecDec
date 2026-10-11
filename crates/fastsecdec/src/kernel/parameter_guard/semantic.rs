//! Canonical predicate identity, separate from native program/State bytes.
use super::*;
use crate::kernel::threshold_metadata::{
    Digest, ParameterPredicateRole as Role, ParameterPredicateV1,
};
use symbolica::id::{Pattern, Replacement};

fn digest(domain: &[u8], value: &impl serde::Serialize) -> Result<String> {
    let mut hash = blake3::Hasher::new();
    hash.update(domain);
    serde_json::to_writer(&mut hash, value).map_err(native)?;
    Ok(hash.finalize().to_hex().to_string())
}
impl Stored {
    fn canonical(&self, index: usize) -> Result<String> {
        let value = self
            .expressions
            .get(index)
            .ok_or(Error::Invalid("predicate output index"))?;
        // Native simultaneous replacement is essential when raw/public names
        // overlap or a permutation swaps symbols. Original physical names stay.
        Ok(value
            .replace_multiple(self.raw_parameters.iter().zip(&self.prefix_to_public).map(
                |(raw, public)| {
                    Replacement::new(
                        Pattern::Literal(Atom::var(*raw)),
                        Pattern::Literal(Atom::var(self.public_parameters[*public])),
                    )
                },
            ))
            .to_canonical_string())
    }
    pub(crate) fn predicate_roles(&self) -> Result<Vec<ParameterPredicateV1>> {
        self.validate(&Limits::default())?;
        let mut roles = Vec::new();
        for (axis, bounds) in self.axes.iter().enumerate() {
            for (bound, role) in [
                (&bounds.lower, Role::LowerBound),
                (&bounds.upper, Role::UpperBound),
            ] {
                if let Some(bound) = bound {
                    roles.push(ParameterPredicateV1 {
                        role,
                        index: axis,
                        positive_selector: bound.positive_selector,
                        predicate: Digest(digest(
                            b"threshold-parameter-bound-v1\0",
                            &(
                                self.canonical(bound.numerator)?,
                                self.canonical(bound.denominator)?,
                            ),
                        )?),
                    });
                }
            }
        }
        for (indices, role) in [
            (&self.positive, Role::StrictPositive),
            (&self.nonzero, Role::Nonzero),
        ] {
            for (index, slot) in indices.iter().enumerate() {
                roles.push(ParameterPredicateV1 {
                    role,
                    index,
                    positive_selector: false,
                    predicate: Digest(digest(
                        b"threshold-parameter-polynomial-v1\0",
                        &self.canonical(*slot)?,
                    )?),
                });
            }
        }
        Ok(roles)
    }
    pub(crate) fn semantic_identity(&self) -> Result<String> {
        // Native source-polynomial numbers are diagnostics. IR optimization,
        // symbol registration, transport grouping and those numbers are absent.
        let names = self
            .public_parameters
            .iter()
            .map(|s| Atom::var(*s).to_canonical_string())
            .collect::<Vec<_>>();
        digest(
            b"threshold-exact-parameter-guard-v1\0",
            &(names, &self.prefix_to_public, self.predicate_roles()?),
        )
    }
}
impl ExactParameterGuard {
    pub(crate) fn semantic_identity(&self) -> Result<String> {
        self.stored.semantic_identity()
    }
    pub(crate) fn predicate_roles(&self) -> Result<Vec<ParameterPredicateV1>> {
        self.stored.predicate_roles()
    }
}

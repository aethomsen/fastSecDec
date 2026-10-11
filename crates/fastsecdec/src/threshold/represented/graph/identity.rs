use super::*;
use crate::generation::identity::{CanonicalAtom, CanonicalSymbol};
use serde::Serialize;

fn item(hash: &mut Vec<u8>, value: &impl Serialize) -> Result<()> {
    serde_json::to_writer(&mut *hash, value).map_err(|error| Error::Invalid(error.to_string()))?;
    hash.push(0);
    Ok(())
}

pub(super) fn witness(owner: &ExactRepresentedGraphInput) -> Result<String> {
    let mut hash = if owner.payload_policy.is_some() {
        b"fastsecdec-preparametric-represented-graph-payload-v2\0".to_vec()
    } else {
        b"fastsecdec-preparametric-represented-graph-v1\0".to_vec()
    };
    if let Some(policy) = owner.payload_policy {
        item(&mut hash, &policy)?;
        item(&mut hash, owner.exact_diagram.as_ref())?;
    }
    let point = &owner.original;
    // Existing native graph serialization retains model fingerprint, topology,
    // routes, cuts, fragments and all weights. No alternate graph DTO is built.
    item(&mut hash, point.diagram.as_ref())?;
    item(&mut hash, &owner.meaning)?;
    item(
        &mut hash,
        &CanonicalAtom(&point.kinematics.dimension().to_symbolic()),
    )?;
    let mut momenta = point
        .kinematics
        .momenta()
        .map(AtomCore::to_canonical_string)
        .collect::<Vec<_>>();
    momenta.sort();
    item(&mut hash, &momenta)?;
    let mut assumptions = point
        .kinematics
        .scalar_assumptions()
        .map(|(key, value)| (key.to_canonical_string(), value.to_canonical_string()))
        .collect::<Vec<_>>();
    assumptions.sort();
    item(&mut hash, &assumptions)?;
    let mut bindings = point
        .scalar_values
        .iter()
        .map(|(key, value)| {
            (
                Atom::var(*key).to_canonical_string(),
                value.to_canonical_string(),
            )
        })
        .collect::<Vec<_>>();
    bindings.sort();
    item(&mut hash, &bindings)?;
    item(
        &mut hash,
        &point
            .auxiliary_momenta
            .iter()
            .map(CanonicalAtom)
            .collect::<Vec<_>>(),
    )?;
    item(
        &mut hash,
        &point
            .powers
            .iter()
            .map(|(edge, power)| (edge.0, *power))
            .collect::<Vec<_>>(),
    )?;
    item(&mut hash, &CanonicalAtom(&point.measure_multiplier))?;
    item(&mut hash, &CanonicalAtom(&point.dimension))?;
    item(&mut hash, &CanonicalSymbol(point.regulator))?;
    item(
        &mut hash,
        &point
            .coordinates
            .iter()
            .copied()
            .map(CanonicalSymbol)
            .collect::<Vec<_>>(),
    )?;
    for row in &owner.conversions {
        item(
            &mut hash,
            &(
                &row.location,
                CanonicalAtom(&row.original),
                CanonicalAtom(&row.exact),
                row.precision_bits,
                row.binary_exponents,
            ),
        )?;
    }
    item(
        &mut hash,
        &crate::generation::source_identity(&owner.exact, &[], &[])
            .map_err(|error| Error::Invalid(error.to_string()))?,
    )?;
    String::from_utf8(hash).map_err(|error| Error::Invalid(error.to_string()))
}

use super::*;
use crate::generation::identity::{CanonicalAtom, CanonicalSymbol};
use serde::Serialize;

fn item(bytes: &mut Vec<u8>, value: &impl Serialize) -> Result<()> {
    serde_json::to_writer(&mut *bytes, value).map_err(|e| Error::Invalid(e.to_string()))?;
    bytes.push(0);
    Ok(())
}
fn atoms(bytes: &mut Vec<u8>, values: &[Atom]) -> Result<()> {
    item(bytes, &values.iter().map(CanonicalAtom).collect::<Vec<_>>())
}
fn kinematics(bytes: &mut Vec<u8>, owner: &Kinematics) -> Result<()> {
    item(bytes, &CanonicalAtom(&owner.dimension().to_symbolic()))?;
    let mut momenta = owner
        .momenta()
        .map(AtomCore::to_canonical_string)
        .collect::<Vec<_>>();
    momenta.sort();
    item(bytes, &momenta)?;
    let mut assumptions = owner
        .scalar_assumptions()
        .map(|(k, v)| (k.to_canonical_string(), v.to_canonical_string()))
        .collect::<Vec<_>>();
    assumptions.sort();
    item(bytes, &assumptions)
}

pub(super) fn witness(owner: &ExactRepresentedFamilyInput) -> Result<String> {
    let mut bytes = b"fastsecdec-preparametric-represented-family-v1\0".to_vec();
    let point = &owner.original;
    let family = &point.family;
    item(&mut bytes, &owner.meaning)?;
    kinematics(&mut bytes, family.kinematics())?;
    atoms(&mut bytes, family.loop_momenta())?;
    atoms(&mut bytes, family.external_momenta())?;
    atoms(&mut bytes, family.scalar_products())?;
    atoms(&mut bytes, family.denominators())?;
    item(&mut bytes, &family.rank())?;
    item(&mut bytes, &point.kinematics.is_some())?;
    if let Some(kinematics_owner) = &point.kinematics {
        kinematics(&mut bytes, kinematics_owner)?;
    }
    item(&mut bytes, &point.powers)?;
    item(&mut bytes, &CanonicalAtom(&point.numerator))?;
    item(&mut bytes, &CanonicalAtom(&point.measure_multiplier))?;
    let mut bindings = point
        .scalar_values
        .iter()
        .map(|(k, v)| (Atom::var(*k).to_canonical_string(), v.to_canonical_string()))
        .collect::<Vec<_>>();
    bindings.sort();
    item(&mut bytes, &bindings)?;
    atoms(&mut bytes, &point.auxiliary_momenta)?;
    item(&mut bytes, &CanonicalAtom(&point.dimension))?;
    item(&mut bytes, &CanonicalSymbol(point.regulator))?;
    item(
        &mut bytes,
        &point
            .coordinates
            .iter()
            .copied()
            .map(CanonicalSymbol)
            .collect::<Vec<_>>(),
    )?;
    for row in &owner.conversions {
        item(
            &mut bytes,
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
        &mut bytes,
        &crate::generation::source_identity(&owner.exact, &[], &[])
            .map_err(|e| Error::Invalid(e.to_string()))?,
    )?;
    String::from_utf8(bytes).map_err(|e| Error::Invalid(e.to_string()))
}

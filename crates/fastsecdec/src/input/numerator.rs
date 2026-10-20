use feynkit_graph::FeynmanDiagram;
use feynkit_kinematics::Kinematics;
use idenso::tensor::{AlgebraContraction, AlgebraSettings, SymbolicTensor};
use spenso::structure::TensorStructure;
use symbolica::atom::Atom;

use crate::{Error, Result};

/// Use HEPKit's established identity families while retaining unrelated scalar
/// factorization. No vacuum tensor averaging or external Gram inverse is used.
pub fn default_algebra_settings() -> AlgebraSettings {
    AlgebraSettings {
        contract: AlgebraContraction::Minimal,
        ..AlgebraSettings::hep()
    }
}

pub fn contract_numerator(
    diagram: &FeynmanDiagram,
    kinematics: &Kinematics,
    settings: &AlgebraSettings,
) -> Result<Atom> {
    let overall = SymbolicTensor::infer(diagram.overall_factor().clone())
        .map_err(|error| Error::Tensor(error.to_string()))?;
    let projected = diagram
        .numerator()
        .multiply(diagram.projector())
        .and_then(|tensor| tensor.multiply(diagram.numerator_prefactor()))
        .and_then(|tensor| tensor.multiply(&overall))
        .map_err(|error| Error::Tensor(error.to_string()))?
        .with_lorentz_dimension(kinematics.dimension())
        .map_err(|error| Error::Tensor(error.to_string()))?;
    let routed = diagram
        .loop_momentum_basis()
        .route_expression(projected.expression());
    let tensor = projected
        .with_checked_expression(routed)
        .and_then(|tensor| tensor.simplify_algebra(settings))
        .and_then(|tensor| tensor.to_dots())
        .map_err(|error| Error::Tensor(error.to_string()))?;
    if !tensor.structure().canonical().is_scalar() {
        return Err(Error::FreeTensorIndices);
    }
    Ok(kinematics.apply(tensor.expression()))
}

//! Faithful explicit point conversion for an already exact native family.
//! Earlier numerical arithmetic in a supplied family cannot be recovered here.
mod identity;

use super::{Error, Limits, LiteralConversion, NumericalMeaning, Result, Work};
use crate::{Atom, AtomCore, IntegralFamily, Kinematics, Symbol, parametric::ParametricIntegrand};
use std::{collections::BTreeMap, ops::ControlFlow, sync::Arc};

/// Native family and untouched point inputs, before numerator/measure products
/// or family specialization. Construction performs no numerical arithmetic.
#[derive(Clone, Debug)]
pub struct FamilyPoint {
    pub family: Arc<IntegralFamily>,
    pub powers: Vec<i32>,
    pub numerator: Atom,
    pub measure_multiplier: Atom,
    pub kinematics: Option<Arc<Kinematics>>,
    pub scalar_values: BTreeMap<Symbol, Atom>,
    pub auxiliary_momenta: Vec<Atom>,
    pub coordinates: Vec<Symbol>,
    pub regulator: Symbol,
    pub dimension: Atom,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FamilyLocation {
    KinematicValue {
        keys: Vec<String>,
    },
    ScalarBinding {
        symbol: String,
    },
    Numerator {},
    Measure {},
    /// Admission scan only; represented payloads in the stored basis refuse.
    ExactPayload {
        role: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    ExactFamily,
    Kinematics,
    ScalarBindings,
    Numerator,
    Measure,
    Parameterization,
    Complete,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Progress {
    pub stage: Stage,
    pub converted_literals: usize,
}

/// Fixed represented point with complete original native family provenance.
/// No geometry/continuation authority is issued by conversion.
#[derive(Clone, Debug)]
pub struct ExactRepresentedFamilyInput {
    original: Arc<FamilyPoint>,
    exact_kinematics: Arc<Kinematics>,
    exact_scalar_values: BTreeMap<Symbol, Atom>,
    exact_numerator: Atom,
    exact_measure: Atom,
    exact: Arc<ParametricIntegrand>,
    conversions: Vec<LiteralConversion<FamilyLocation>>,
    meaning: NumericalMeaning,
    limits: Limits,
    source_identity: String,
    source_witness: String,
}
impl PartialEq for ExactRepresentedFamilyInput {
    fn eq(&self, other: &Self) -> bool {
        self.source_witness == other.source_witness
            && self.exact == other.exact
            && self.limits == other.limits
    }
}
impl Eq for ExactRepresentedFamilyInput {}

fn require_exact(work: &mut Work<FamilyLocation>, atom: &Atom, role: &str) -> Result<()> {
    let before = work.conversions.len();
    let _ = work.atom(atom, FamilyLocation::ExactPayload { role: role.into() })?;
    if work.conversions.len() != before {
        return Err(Error::Unsupported(
            "stored family basis/context must be exact; earlier Float arithmetic cannot be recovered",
        ));
    }
    Ok(())
}

impl ExactRepresentedFamilyInput {
    pub fn original(&self) -> &Arc<FamilyPoint> {
        &self.original
    }
    pub fn exact_kinematics(&self) -> &Arc<Kinematics> {
        &self.exact_kinematics
    }
    pub fn exact_scalar_values(&self) -> &BTreeMap<Symbol, Atom> {
        &self.exact_scalar_values
    }
    pub fn exact_numerator(&self) -> &Atom {
        &self.exact_numerator
    }
    pub fn exact_measure(&self) -> &Atom {
        &self.exact_measure
    }
    pub fn exact(&self) -> &Arc<ParametricIntegrand> {
        &self.exact
    }
    pub fn conversions(&self) -> &[LiteralConversion<FamilyLocation>] {
        &self.conversions
    }
    pub fn meaning(&self) -> NumericalMeaning {
        self.meaning
    }
    pub fn limits(&self) -> Limits {
        self.limits
    }
    pub fn source_identity(&self) -> &str {
        &self.source_identity
    }
    pub(crate) fn source_witness(&self) -> &str {
        &self.source_witness
    }

    /// Convert each point slot before arithmetic, then delegate specialization,
    /// signed powers, sector projection and parameterization to native owners.
    /// Resource limits bound conversion, not native parameterization process RSS.
    pub fn prepare(
        original: Arc<FamilyPoint>,
        meaning: NumericalMeaning,
        limits: Limits,
        mut observer: impl FnMut(Progress) -> ControlFlow<()>,
    ) -> Result<Self> {
        let mut work = Work::new(meaning, limits)?;
        let mut observe = |stage, work: &Work<FamilyLocation>| {
            if observer(Progress {
                stage,
                converted_literals: work.conversions.len(),
            })
            .is_break()
            {
                Err(Error::Cancelled)
            } else {
                Ok(())
            }
        };
        observe(Stage::ExactFamily, &work)?;
        let family = &original.family;
        require_exact(&mut work, &original.dimension, "integration_dimension")?;
        require_exact(
            &mut work,
            &family.kinematics().dimension().to_symbolic(),
            "tensor_dimension",
        )?;
        for (role, atoms) in [
            ("loop_momentum", family.loop_momenta()),
            ("external_momentum", family.external_momenta()),
            ("scalar_product_basis", family.scalar_products()),
            ("denominator", family.denominators()),
            ("auxiliary_momentum", original.auxiliary_momenta.as_slice()),
        ] {
            for atom in atoms {
                require_exact(&mut work, atom, role)?;
            }
        }
        for momentum in family.kinematics().momenta() {
            require_exact(&mut work, momentum, "stored_momentum")?;
        }
        for (key, value) in family.kinematics().scalar_assumptions() {
            require_exact(&mut work, key, "stored_assumption_key")?;
            require_exact(&mut work, value, "stored_assumption_value")?;
        }
        let point = original
            .kinematics
            .as_deref()
            .unwrap_or_else(|| family.kinematics());
        require_exact(
            &mut work,
            &point.dimension().to_symbolic(),
            "point_tensor_dimension",
        )?;
        for momentum in point.momenta() {
            require_exact(&mut work, momentum, "point_momentum")?;
        }
        observe(Stage::Kinematics, &work)?;
        let mut associations = BTreeMap::<&Atom, Vec<String>>::new();
        for (key, value) in point.scalar_assumptions() {
            require_exact(&mut work, key, "point_assumption_key")?;
            associations
                .entry(value)
                .or_default()
                .push(key.to_canonical_string());
        }
        for keys in associations.values_mut() {
            keys.sort();
        }
        let exact_kinematics = Arc::new(point.try_map_scalar_values(|value| {
            work.atom(
                value,
                FamilyLocation::KinematicValue {
                    keys: associations[value].clone(),
                },
            )
        })?);
        observe(Stage::ScalarBindings, &work)?;
        let exact_scalar_values = original
            .scalar_values
            .iter()
            .map(|(symbol, value)| {
                work.atom(
                    value,
                    FamilyLocation::ScalarBinding {
                        symbol: Atom::var(*symbol).to_canonical_string(),
                    },
                )
                .map(|value| (*symbol, value))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        observe(Stage::Numerator, &work)?;
        let exact_numerator = work.atom(&original.numerator, FamilyLocation::Numerator {})?;
        observe(Stage::Measure, &work)?;
        let exact_measure = work.atom(&original.measure_multiplier, FamilyLocation::Measure {})?;
        observe(Stage::Parameterization, &work)?;
        let (prepared, powers, numerator) = crate::input::prepare_family_input(
            family,
            &original.powers,
            &exact_numerator * &exact_measure,
            Some(&exact_kinematics),
            &exact_scalar_values,
            &original.auxiliary_momenta,
        )
        .map_err(|error| Error::Invalid(error.to_string()))?;
        let exact = Arc::new(
            ParametricIntegrand::from_family(
                &prepared,
                &powers,
                numerator,
                original.coordinates.clone(),
                original.regulator,
                original.dimension.clone(),
            )
            .map_err(|error| Error::Invalid(error.to_string()))?,
        );
        work.conversions.sort_by_cached_key(|row| {
            (
                row.location.clone(),
                row.original.to_canonical_string(),
                row.exact.to_canonical_string(),
                row.precision_bits,
                row.binary_exponents,
            )
        });
        let mut result = Self {
            original,
            exact_kinematics,
            exact_scalar_values,
            exact_numerator,
            exact_measure,
            exact,
            conversions: work.conversions,
            meaning,
            limits,
            source_identity: String::new(),
            source_witness: String::new(),
        };
        result.source_witness = identity::witness(&result)?;
        result.source_identity = blake3::hash(result.source_witness.as_bytes())
            .to_hex()
            .to_string();
        if observer(Progress {
            stage: Stage::Complete,
            converted_literals: result.conversions.len(),
        })
        .is_break()
        {
            return Err(Error::Cancelled);
        }
        Ok(result)
    }
}

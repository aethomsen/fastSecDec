//! Four-dimensional external data; internal tensor algebra stays in D dimensions.

use std::collections::{BTreeMap, HashMap};

use feynkit_graph::{ExternalState, FeynmanDiagram, symbols};
use feynkit_kinematics::{FourMomentum, Helicity, WavefunctionKind};
use numerica::domains::{float::Complex, rational::Rational};
use serde::Serialize;
use symbolica::{
    atom::{Atom, AtomCore},
    parser::ParseSettings,
};

use super::Result;

pub fn auxiliaries() -> [Atom; 2] {
    [
        Atom::var(spenso::vector_symbol!("gghh::eps1")),
        Atom::var(spenso::vector_symbol!("gghh::eps2")),
    ]
}

pub fn expression(text: &str) -> Result<Atom> {
    Ok(Atom::parse(text, "gghh", ParseSettings::default())?)
}

#[derive(Serialize)]
pub struct Product {
    pub left: String,
    pub right: String,
    pub value: String,
}

#[derive(Serialize)]
pub struct Point {
    pub products: Vec<Product>,
    pub external_index_by_coordinate: Vec<usize>,
    pub physical_momenta: BTreeMap<usize, [String; 4]>,
    pub polarization_components: Vec<Vec<[f64; 2]>>,
    pub independent_coordinates: Vec<String>,
    pub dependent_coordinates: Vec<String>,
    pub polarization_gram_checks_complete: bool,
}

fn from_components(x: &[Complex<f64>]) -> Result<FourMomentum<Atom>> {
    let atom = |z: &Complex<f64>| -> Result<Atom> {
        // Preserve the supplied binary f64 components exactly in the native
        // rational coefficient domain required by polynomial parameterization.
        Ok(Atom::num(Rational::try_from(z.re)?) + Atom::num(Rational::try_from(z.im)?) * Atom::i())
    };
    Ok(FourMomentum::from_args(
        atom(&x[0])?,
        atom(&x[1])?,
        atom(&x[2])?,
        atom(&x[3])?,
    ))
}

fn constant(atom: &Atom) -> Result<Complex<f64>> {
    let value = atom.evaluate(&HashMap::<Atom, Complex<f64>>::new())?;
    if !value.re.is_finite() || !value.im.is_finite() {
        return Err("external Gram entry is not a finite numerical constant".into());
    }
    Ok(value)
}

/// Match runtime binding's native f64 evaluation, then export a TOML float.
pub fn real_value(expression: &str) -> Result<f64> {
    let value = self::expression(expression)?.evaluate(&HashMap::<Atom, f64>::new())?;
    if !value.is_finite() {
        return Err("external Gram entry is not a finite real constant".into());
    }
    Ok(value)
}

fn near(atom: &Atom, target: f64) -> Result<()> {
    let value = constant(atom)?;
    if (value.re - target).abs() > 2e-12 || value.im.abs() > 2e-12 {
        return Err(
            format!("external polarization Gram check failed: {value:?} != {target}").into(),
        );
    }
    Ok(())
}

impl Point {
    pub fn new(diagram: &FeynmanDiagram) -> Result<Self> {
        Self::with_sqrt_s(diagram, 300)
    }

    /// Shared exact kinematics for the below- and above-top-threshold controls.
    /// The Higgs mass and scattering angle remain 125 GeV and cos(theta)=4/5.
    pub fn with_sqrt_s(diagram: &FeynmanDiagram, sqrt_s_gev: u32) -> Result<Self> {
        if sqrt_s_gev <= 250 {
            return Err("sqrt(s) must exceed the two-Higgs threshold of 250 GeV".into());
        }
        let energy = Atom::num((i64::from(sqrt_s_gev), 2));
        let spatial = (energy.pow(2) - Atom::num(125 * 125)).sqrt();
        Self::with_spatial_components(
            diagram,
            sqrt_s_gev,
            &spatial * Atom::num((3, 5)),
            spatial * Atom::num((4, 5)),
        )
    }

    /// Exact physical point with mH=125 and t=(p_in[0]-p_out[0])².
    /// Incoming/outgoing labels are sorted as in `with_sqrt_s`. The outgoing
    /// transverse momentum has the same positive-x convention. Wavefunctions
    /// and every Gram entry still come from the shared native primitives.
    #[allow(dead_code, reason = "shared with the explicit-point input exporter")]
    pub fn with_mandelstam_t(
        diagram: &FeynmanDiagram,
        sqrt_s_gev: u32,
        t: Rational,
    ) -> Result<Self> {
        if sqrt_s_gev <= 250 {
            return Err("sqrt(s) must exceed the two-Higgs threshold of 250 GeV".into());
        }
        let energy = Rational::from((i64::from(sqrt_s_gev), 2));
        let energy_squared = &energy * &energy;
        let mass_squared = Rational::from(125 * 125);
        let longitudinal = (t - &mass_squared + &energy_squared * &Rational::from(2))
            / Rational::from(i64::from(sqrt_s_gev));
        let transverse_squared = energy_squared - mass_squared - &longitudinal * &longitudinal;
        if transverse_squared < 0 {
            return Err("Mandelstam t is outside the physical two-Higgs angular range".into());
        }
        Self::with_spatial_components(
            diagram,
            sqrt_s_gev,
            Atom::num(transverse_squared).sqrt(),
            Atom::num(longitudinal),
        )
    }

    fn with_spatial_components(
        diagram: &FeynmanDiagram,
        sqrt_s_gev: u32,
        px: Atom,
        pz: Atom,
    ) -> Result<Self> {
        if sqrt_s_gev <= 250 {
            return Err("sqrt(s) must exceed the two-Higgs threshold of 250 GeV".into());
        }
        let mut incoming = Vec::new();
        let mut outgoing = Vec::new();
        let external_by_edge = diagram
            .edges()
            .filter_map(|(id, _, edge)| edge.external.as_ref().map(|external| (id, external)))
            .collect::<BTreeMap<_, _>>();
        for external in external_by_edge.values() {
            match external.state {
                ExternalState::Incoming => incoming.push(external.index),
                ExternalState::Outgoing => outgoing.push(external.index),
            }
        }
        incoming.sort_unstable();
        outgoing.sort_unstable();
        if incoming.len() != 2 || outgoing.len() != 2 {
            return Err("expected gg -> HH external states".into());
        }
        // Native exact arithmetic keeps the on-shell and conservation
        // identities exact. Helicity vectors come from the shared primitive.
        let energy = Atom::num((i64::from(sqrt_s_gev), 2));
        let vectors = [
            FourMomentum::from_args(energy.clone(), Atom::Zero, Atom::Zero, energy.clone()),
            FourMomentum::from_args(energy.clone(), Atom::Zero, Atom::Zero, -&energy),
            FourMomentum::from_args(energy.clone(), px.clone(), Atom::Zero, pz.clone()),
            FourMomentum::from_args(energy, -px, Atom::Zero, -pz),
        ];
        let physical = incoming
            .iter()
            .chain(&outgoing)
            .copied()
            .zip(vectors)
            .collect::<BTreeMap<_, _>>();
        let basis = diagram.loop_momentum_basis();
        let labels = basis
            .external_edges
            .iter()
            .map(|edge| external_by_edge[edge].index)
            .collect::<Vec<_>>();
        let coordinate_vectors = labels
            .iter()
            .map(|id| physical[id].clone())
            .collect::<Vec<_>>();
        for edge in &basis.external_edges {
            let routed = basis.edge_signatures[edge]
                .external
                .apply(&coordinate_vectors)?
                .ok_or("zero external routing")?;
            if routed
                .components()
                .into_iter()
                .zip(physical[&external_by_edge[edge].index].components())
                .any(|(a, b)| !(a - b).expand().is_zero())
            {
                return Err(
                    "native external routing disagrees with physical momentum conservation".into(),
                );
            }
        }
        let energy = f64::from(sqrt_s_gev) / 2.0;
        let wavefunctions = [energy, -energy]
            .into_iter()
            .map(|z| {
                FourMomentum::from_args(energy, 0.0, 0.0, z)
                    .wavefunction(WavefunctionKind::Epsilon, Helicity::PLUS)
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let polarizations = wavefunctions
            .iter()
            .map(|w| from_components(w.components()))
            .collect::<Result<Vec<_>>>()?;
        for (index, (state, epsilon)) in wavefunctions.iter().zip(&polarizations).enumerate() {
            let null = epsilon.dot(epsilon).expand();
            if !null.is_zero() {
                return Err(
                    "circular polarization must be exactly null after native complex transport"
                        .into(),
                );
            }
            near(
                &epsilon.dot(&from_components(state.bar().components())?),
                -1.0,
            )?;
            near(&physical[&incoming[index]].dot(epsilon), 0.0)?;
        }
        near(&polarizations[0].dot(&polarizations[1]), -1.0)?;
        let mut named = Vec::new();
        let mut independent = Vec::new();
        let mut dependent = Vec::new();
        for (i, edge) in basis.external_edges.iter().enumerate() {
            let name = symbols::external_momentum().call(i).to_canonical_string();
            if basis.dependent_externals.contains(edge) {
                dependent.push(name);
            } else {
                independent.push(name.clone());
                named.push((name, coordinate_vectors[i].clone()));
            }
        }
        for (name, epsilon) in auxiliaries().into_iter().zip(polarizations) {
            named.push((name.to_canonical_string(), epsilon));
        }
        let mut products = Vec::new();
        for (i, (left, a)) in named.iter().enumerate() {
            for (right, b) in &named[i..] {
                let value = a.dot(b).expand();
                constant(&value)?;
                products.push(Product {
                    left: left.clone(),
                    right: right.clone(),
                    value: value.to_canonical_string(),
                });
            }
        }
        Ok(Self {
            products,
            external_index_by_coordinate: labels,
            physical_momenta: physical
                .into_iter()
                .map(|(id, p)| (id, p.components().map(|x| x.to_canonical_string())))
                .collect(),
            polarization_components: wavefunctions
                .iter()
                .map(|w| w.components().iter().map(|z| [z.re, z.im]).collect())
                .collect(),
            independent_coordinates: independent,
            dependent_coordinates: dependent,
            polarization_gram_checks_complete: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exported_float_preserves_native_runtime_value() -> Result<()> {
        for text in ["0", "45000", "1/3", "-22500+3000*11^(1/2)"] {
            let expected = expression(text)?.evaluate(&HashMap::<Atom, f64>::new())?;
            let literal = format!("{:?}", real_value(text)?);
            assert_eq!(literal.parse::<f64>()?.to_bits(), expected.to_bits());
            assert!(literal.contains('.') || literal.contains('e'));
        }
        Ok(())
    }

    #[test]
    fn point_export_rejects_complex_inputs() {
        assert!(real_value(&(Atom::one() + Atom::i()).to_canonical_string()).is_err());
    }
}

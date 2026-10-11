use super::super::{
    EndpointTransport, FactorLink,
    clearing::{ClearedFactor, Ring, pull},
};
use super::{Error, Limits, NumeratorLink, Result, SourceNumerators, Work, native};
use crate::{generation::add_scaled_powers, threshold::maps::regular::Budget};
use std::{collections::BTreeSet, ops::ControlFlow, sync::Arc};
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    domains::{integer::Integer, rational::Rational},
    poly::series::SeriesDepth,
};

/// One original numerator's finite Taylor coefficients after exact extraction
/// of its normal power. The leading coefficient need not be a closed unit.
#[derive(Debug)]
pub struct FaceCoefficient {
    source_record: usize,
    clearing: Atom,
    direction_divisor: Atom,
    cleared_order: Integer,
    normal_order: Integer,
    coefficients: Vec<Atom>,
    exact_zero: bool,
}
impl FaceCoefficient {
    pub fn source_record(&self) -> usize {
        self.source_record
    }
    pub fn clearing(&self) -> &Atom {
        &self.clearing
    }
    pub fn direction_divisor(&self) -> &Atom {
        &self.direction_divisor
    }
    pub fn cleared_order(&self) -> &Integer {
        &self.cleared_order
    }
    pub fn normal_order(&self) -> &Integer {
        &self.normal_order
    }
    /// Coefficients of the residual numerator; derivative j also needs j!.
    pub fn coefficients(&self) -> &[Atom] {
        &self.coefficients
    }
    pub fn exact_zero(&self) -> bool {
        self.exact_zero
    }
}
/// Source-bound formal coefficients on one relative open lower face.
#[derive(Debug)]
pub struct NumeratorFace {
    catalogue: Arc<SourceNumerators>,
    transport: Arc<EndpointTransport>,
    chart_index: usize,
    normal: Symbol,
    depth: u32,
    finite_coordinates: Vec<Symbol>,
    finite_images: Vec<Atom>,
    direction_units: Vec<Symbol>,
    root_symbols: Vec<Symbol>,
    coefficients: Vec<FaceCoefficient>,
    record_to_coefficient: Vec<Option<usize>>,
    term_powers: Vec<Vec<Atom>>,
}
impl NumeratorFace {
    pub fn catalogue(&self) -> &Arc<SourceNumerators> {
        &self.catalogue
    }
    pub fn transport(&self) -> &Arc<EndpointTransport> {
        &self.transport
    }
    pub fn chart_index(&self) -> usize {
        self.chart_index
    }
    pub fn normal(&self) -> Symbol {
        self.normal
    }
    pub fn depth(&self) -> u32 {
        self.depth
    }
    /// Retained independent source formals, in genuine preparation order.
    pub fn finite_coordinates(&self) -> &[Symbol] {
        &self.finite_coordinates
    }
    pub fn finite_images(&self) -> &[Atom] {
        &self.finite_images
    }
    pub fn direction_units(&self) -> &[Symbol] {
        &self.direction_units
    }
    /// Actual selected root roles, proved independent of this normal in prefix order.
    pub fn root_symbols(&self) -> &[Symbol] {
        &self.root_symbols
    }
    pub fn coefficients(&self) -> &[FaceCoefficient] {
        &self.coefficients
    }
    pub fn record_to_coefficient(&self) -> &[Option<usize>] {
        &self.record_to_coefficient
    }
    pub fn term_powers(&self) -> &[Vec<Atom>] {
        &self.term_powers
    }
    pub fn closed_density_certified(&self) -> bool {
        false
    }

    /// Only retained finite source coordinates are independent formals here.
    /// An eliminated source axis needs differentiation BEFORE substitution.
    pub fn source_partial(
        &self,
        coefficient: usize,
        normal_order: usize,
        coordinate: Symbol,
        budget: &Budget,
    ) -> Result<Atom> {
        budget.check()?;
        if !self.finite_coordinates.contains(&coordinate) {
            return Err(Error::Unsupported(
                "source partial requires a retained independent finite formal".into(),
            ));
        }
        let body = self
            .coefficients
            .get(coefficient)
            .and_then(|c| c.coefficients.get(normal_order))
            .ok_or_else(|| Error::Invalid("finite coefficient index".into()))?;
        let derivative = body.derivative(coordinate);
        budget.check()?;
        Ok(derivative)
    }
    pub fn prepare(
        catalogue: Arc<SourceNumerators>,
        transport: Arc<EndpointTransport>,
        chart_index: usize,
        normal: Symbol,
        depth: u32,
        limits: &Limits,
        mut progress: impl FnMut() -> ControlFlow<()>,
    ) -> Result<Arc<Self>> {
        let mut work = Work {
            limits,
            budget: Budget::new(limits.algebra.clone()),
            callback: &mut progress,
        };
        work.check()?;
        let preparation = transport
            .parent_prefix()
            .source()
            .decomposition()
            .preparation();
        if !Arc::ptr_eq(catalogue.preparation(), preparation) || !transport.transport_complete() {
            return Err(Error::Invalid(
                "foreign numerator source or incomplete transport".into(),
            ));
        }
        let chart = transport
            .charts()
            .get(chart_index)
            .ok_or_else(|| Error::Invalid("foreign native chart ordinal".into()))?;
        let axis = transport
            .target_units()
            .iter()
            .position(|s| *s == normal)
            .ok_or_else(|| Error::Invalid("foreign normal role".into()))?;
        if depth == 0 || depth > limits.max_relative_depth {
            return Err(Error::ResourceIncomplete(
                "finite relative series depth".into(),
            ));
        }
        let normal_atom = Atom::var(normal);
        let root_symbols = independent_roots(&transport, chart_index, normal, &mut work)?;
        let roles = transport
            .target_units()
            .iter()
            .chain(&root_symbols)
            .copied()
            .collect::<Vec<_>>();
        let mut ring = Ring::new(roles)?;
        ring.reserve(
            catalogue
                .records()
                .iter()
                .flat_map(|r| r.body().get_all_symbols(true).into_iter()),
        );
        let mut finite_coordinates = Vec::new();
        let mut finite_images = Vec::new();
        let mut changed = Vec::new();
        let mut images = Vec::new();
        let mut denominators = Vec::new();
        let mut denominator_orders = Vec::new();
        let geometric_limits = super::super::Limits {
            algebra: limits.algebra.clone(),
            max_entries: limits.max_entries,
            ..Default::default()
        };
        for (i, (&s, image)) in preparation
            .coordinates()
            .iter()
            .zip(chart.source_images())
            .enumerate()
        {
            work.check()?;
            if !image.contains(normal_atom.as_view()) {
                finite_coordinates.push(s);
                finite_images.push(image.clone());
                continue;
            }
            let mut geometry_work = super::super::Work {
                limits: &geometric_limits,
                budget: Budget::new(limits.algebra.clone()),
                callback: &mut *work.callback,
            };
            let identity =
                canonical_image(&ring, image, transport.target_units(), &mut geometry_work)?;
            changed.push((i, s));
            // Native canonical geometry has only the proved monomial inverse.
            // Do not carry cancellable inverses from the original syntax into
            // the compact numerator series.
            images.push(identity.numerator() / identity.denominator());
            denominators.push(identity.denominator().clone());
            denominator_orders.push(
                identity
                    .denominator_powers()
                    .iter()
                    .map(|p| p[0])
                    .collect::<Vec<_>>(),
            );
        }
        let direction_units = transport
            .target_units()
            .iter()
            .copied()
            .filter(|s| *s != normal)
            .collect::<Vec<_>>();
        let mut coefficients = Vec::new();
        let mut record_to_coefficient = vec![None; catalogue.records().len()];
        work.entries(
            catalogue
                .records()
                .len()
                .checked_mul(depth as usize)
                .ok_or_else(|| {
                    Error::ResourceIncomplete("finite coefficient entry overflow".into())
                })?,
        )?;
        for (record, source) in catalogue.records().iter().enumerate() {
            let Some(degrees) = source.degrees() else {
                continue;
            };
            work.check()?;
            if degrees.len() != preparation.coordinates().len() {
                return Err(Error::Invalid("native source degree inventory".into()));
            }
            let mut orders = vec![0u32; transport.target_units().len()];
            let mut clearing = Atom::one();
            for ((&(i, _), denominator), powers) in
                changed.iter().zip(&denominators).zip(&denominator_orders)
            {
                clearing *= denominator.pow(degrees[i]);
                for (order, p) in orders.iter_mut().zip(powers) {
                    *order = order
                        .checked_add(p.checked_mul(degrees[i]).ok_or_else(|| {
                            Error::ResourceIncomplete("source clearing degree overflow".into())
                        })?)
                        .ok_or_else(|| {
                            Error::ResourceIncomplete("source clearing degree overflow".into())
                        })?;
                }
            }
            let degree = orders
                .iter()
                .try_fold(0usize, |a, b| a.checked_add(*b as usize))
                .ok_or_else(|| {
                    Error::ResourceIncomplete("clearing total degree overflow".into())
                })?;
            if degree > limits.algebra.max_degree {
                return Err(Error::ResourceIncomplete(
                    "source clearing total degree".into(),
                ));
            }
            let k = orders[axis];
            let direction_divisor = pull(&clearing, &[normal], &[Atom::one()]);
            if direction_divisor.contains(normal_atom.as_view()) {
                return Err(Error::Invalid("normal-dependent clearing residual".into()));
            }
            if !(&clearing - &direction_divisor * normal_atom.clone().pow(k))
                .expand()
                .is_zero()
            {
                return Err(Error::Invalid("native normal clearing identity".into()));
            }
            let (cleared_order, normal_power, values) = if source.exact_zero() {
                (
                    Integer::from(0),
                    Integer::from(0),
                    vec![Atom::zero(); depth as usize],
                )
            } else {
                let body = pull(
                    source.body(),
                    &changed.iter().map(|(_, s)| *s).collect::<Vec<_>>(),
                    &images,
                );
                let h = &clearing * body;
                work.nodes(&h, limits.max_source_nodes)?;
                let series = h
                    .series(normal, Atom::zero(), SeriesDepth::relative(depth))
                    .map_err(native)?;
                work.check()?;
                let v = series.get_trailing_exponent();
                if !v.is_integer() || v.is_negative() {
                    return Err(Error::Unsupported(format!(
                        "nonintegral or negative cleared numerator valuation {v}"
                    )));
                }
                if series.is_zero() || series.relative_order() < depth {
                    return Err(Error::ResourceIncomplete(
                        "native finite series has no complete nonzero coefficient range".into(),
                    ));
                }
                let mut values = Vec::with_capacity(depth as usize);
                for j in 0..depth {
                    let value = series
                        .coefficient(&v + &Rational::from(j))
                        .unwrap_or(Atom::zero())
                        / &direction_divisor;
                    if value.contains(normal_atom.as_view()) {
                        return Err(Error::Invalid(
                            "finite coefficient still depends on its normal".into(),
                        ));
                    }
                    work.nodes(&value, limits.max_coefficient_nodes)?;
                    values.push(value);
                }
                (v.numerator(), v.numerator() - Integer::from(k), values)
            };
            record_to_coefficient[record] = Some(coefficients.len());
            coefficients.push(FaceCoefficient {
                source_record: record,
                clearing,
                direction_divisor,
                cleared_order,
                normal_order: normal_power,
                coefficients: values,
                exact_zero: source.exact_zero(),
            });
        }
        if catalogue.links().len() != chart.terms().len()
            || catalogue.links().len() != transport.term_links().len()
        {
            return Err(Error::Invalid(
                "complete finite numerator term inventory".into(),
            ));
        }
        let mut term_powers = Vec::with_capacity(chart.terms().len());
        for (ti, ((links, term), transported)) in catalogue
            .links()
            .iter()
            .zip(preparation.terms())
            .zip(chart.terms())
            .enumerate()
        {
            work.check()?;
            if links.len() != term.factors().len()
                || links.len() != transport.term_links()[ti].factors().len()
                || transported.source_index() != ti
            {
                return Err(Error::Invalid("finite numerator factor inventory".into()));
            }
            let mut powers = transported.powers().to_vec();
            for (fi, link) in links.iter().enumerate() {
                let source = &term.factors()[fi];
                match link {
                    NumeratorLink::Active { record, exponent } => {
                        if transport.term_links()[ti].factors()[fi] != FactorLink::LazyPolynomial
                            || source.exponent() != &Atom::num(*exponent)
                        {
                            return Err(Error::Invalid(
                                "active numerator factor association".into(),
                            ));
                        }
                        let index = record_to_coefficient
                            .get(*record)
                            .and_then(|x| *x)
                            .ok_or_else(|| {
                                Error::Invalid("missing finite numerator coefficient".into())
                            })?;
                        let mut orders = vec![Integer::from(0); powers.len()];
                        orders[axis] = coefficients[index].normal_order.clone();
                        add_scaled_powers(&mut powers, source.exponent(), &orders)
                            .map_err(native)?;
                    }
                    NumeratorLink::Inactive { .. } if source.exponent().is_zero() => {}
                    NumeratorLink::Geometric
                        if source.role() != crate::parametric::FactorRole::Polynomial => {}
                    _ => return Err(Error::Invalid("finite numerator factor disposition".into())),
                }
            }
            term_powers.push(powers);
        }
        work.check()?;
        Ok(Arc::new(Self {
            catalogue,
            transport,
            chart_index,
            normal,
            depth,
            finite_coordinates,
            finite_images,
            direction_units,
            root_symbols,
            coefficients,
            record_to_coefficient,
            term_powers,
        }))
    }
}

/// Normalize small geometric expressions only. The original numerator is never
/// expanded; it receives the native exact fraction with a proved monomial D.
pub(super) fn canonical_image(
    ring: &Ring,
    image: &Atom,
    units: &[Symbol],
    work: &mut super::super::Work<'_>,
) -> Result<ClearedFactor> {
    use crate::threshold::regularization::meromorphic::{
        Limits as NativeLimits, preflight_expression,
    };
    work.check()?;
    preflight_expression(
        image,
        NativeLimits {
            degree: u16::try_from(work.limits.algebra.max_degree).map_err(native)?,
            terms: work.limits.algebra.max_terms,
            source_nodes: work.limits.algebra.max_terms.saturating_mul(8),
            integer_power: i64::try_from(work.limits.algebra.max_degree).map_err(native)?,
            ..Default::default()
        },
    )
    .map_err(super::preflight_error)?;
    // In particular 1-(1-c) must become c before the unchanged inverse-base
    // admission. Native preflight bounds expansion before it happens.
    let expanded = image.expand();
    work.check()?;
    let identity = ring.clear(&expanded, units, work)?;
    if identity.denominator_powers().iter().any(|p| p[1] != 0) {
        return Err(Error::Unsupported(
            "finite numerator face requires monomial coordinate denominators".into(),
        ));
    }
    Ok(identity)
}

fn independent_roots(
    transport: &EndpointTransport,
    chart_index: usize,
    normal: Symbol,
    work: &mut Work<'_>,
) -> Result<Vec<Symbol>> {
    let normal_atom = Atom::var(normal);
    let all = transport
        .root_relations()
        .iter()
        .map(|r| r.section().root_symbol())
        .collect::<BTreeSet<_>>();
    let mut accepted = BTreeSet::new();
    let mut ordered = Vec::new();
    let parent = transport.parent_prefix().source().unit_coordinates();
    let map = transport.charts()[chart_index].unit_images();
    for relation in transport.root_relations() {
        work.check()?;
        let section = relation.section();
        for image in &section.chart().parameter_images {
            let actual = pull(image, parent, map);
            if actual.contains(normal_atom.as_view())
                || actual
                    .get_all_symbols(true)
                    .iter()
                    .any(|s| all.contains(s) && !accepted.contains(s))
            {
                return Err(Error::Unsupported(
                    "selected-root coefficient inputs depend on normal or unadmitted prior root"
                        .into(),
                ));
            }
        }
        accepted.insert(section.root_symbol());
        ordered.push(section.root_symbol());
    }
    Ok(ordered)
}

//! Delta-method uncertainty of a complex mean's norm (not a stopping criterion).
use super::{CoefficientComponent, VectorEstimate};
use numerica::domains::{
    float::{DoubleFloat, Real, RealLike, SingleFloat},
    rational::Rational,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComplexNormUncertainty {
    pub mean_magnitude: Option<f64>,
    pub radial_standard_error: Option<f64>,
    pub radial_relative_error: Option<f64>,
    pub joint_relative_error: Option<f64>,
    /// Conservative warning heuristic: joint SE / norm >= 0.1, or unavailable.
    /// This is not a mathematical validity boundary for the linearization.
    pub linearization_warning: bool,
}
impl VectorEstimate {
    pub fn complex_norm_uncertainty(&self, order: i32) -> ComplexNormUncertainty {
        let mut result = ComplexNormUncertainty {
            mean_magnitude: None,
            radial_standard_error: None,
            radial_relative_error: None,
            joint_relative_error: None,
            linearization_warning: true,
        };
        if self.validate().is_err() {
            return result;
        }
        let index = |c| {
            self.orders
                .iter()
                .zip(&self.components)
                .position(|(&o, &component)| o == order && component == c)
        };
        let r = index(CoefficientComponent::Real);
        let i = index(CoefficientComponent::Imag);
        if r.is_none() && i.is_none() {
            return result;
        }
        let mean = [
            r.map_or(0.0, |j| self.mean[j]),
            i.map_or(0.0, |j| self.mean[j]),
        ];
        let [real, imag] = mean.map(DoubleFloat::from);
        let norm = real.hypot(&imag);
        let n = norm.to_f64();
        if !n.is_finite() {
            return result;
        }
        result.mean_magnitude = Some(n);
        if n == 0.0 {
            return result;
        }
        let size = self.mean.len();
        let cov = |a: Option<usize>, b: Option<usize>| {
            a.zip(b)
                .map_or(0.0, |(a, b)| self.covariance_of_mean[a * size + b])
        };
        let entries = [cov(r, r), cov(i, i), cov(r, i), cov(i, r)];
        // A finite covariance can have an overflowing trace/projection while
        // its square root is representable. Scale before summing, and restore
        // the scale only after the native square root.
        let scale = entries.iter().fold(0.0_f64, |s, c| s.max(c.abs()));
        let divisor = DoubleFloat::from(if scale == 0.0 { 1.0 } else { scale });
        let [rr, ii, ri, ir] = entries.map(|c| DoubleFloat::from(c) / divisor);
        let root_scale = DoubleFloat::from(scale).sqrt();
        let x = real / norm;
        let y = imag / norm;
        let xx = x * x;
        let xy = x * y;
        let yy = y * y;
        let cross = ri + ir;
        let terms = [xx * rr, xy * cross, yy * ii];
        // DoubleFloat extends significand precision, not exponent range. A
        // global scale can erase a small active covariance beside a large
        // inactive one. Use exact native input arithmetic when a nonzero
        // scaling/product enters the subnormal range.
        let tiny = |v: DoubleFloat| v.to_f64().abs() < f64::MIN_POSITIVE;
        let zero = DoubleFloat::from(0.0);
        let lost_product = |a, b, p| a != zero && b != zero && tiny(p);
        if entries
            .iter()
            .zip([rr, ii, ri, ir])
            .any(|(&a, b)| a != 0.0 && tiny(b))
            || (real != zero && tiny(x))
            || (imag != zero && tiny(y))
            || lost_product(x, x, xx)
            || lost_product(x, y, xy)
            || lost_product(y, y, yy)
            || lost_product(xx, rr, terms[0])
            || lost_product(xy, cross, terms[1])
            || lost_product(yy, ii, terms[2])
        {
            return exact_projection(result, mean, entries);
        }
        let projected = terms[0] + terms[1] + terms[2];
        let trace = rr + ii;
        // This is a conservative algorithm-selection heuristic, not a
        // statement about validity of the statistical linearization.
        let cancellation = |sum: DoubleFloat, absolute: DoubleFloat| {
            absolute > zero && sum.norm() <= DoubleFloat::from(64.0 * f64::EPSILON) * absolute
        };
        let absolute = terms.iter().fold(zero, |sum, t| sum + t.norm());
        if (projected != zero && tiny(projected))
            || (trace != zero && tiny(trace))
            || cancellation(projected, absolute)
            || cancellation(trace, rr.norm() + ii.norm())
        {
            return exact_projection(result, mean, entries);
        }
        if trace.is_finite() && trace >= DoubleFloat::from(0.0) {
            let joint = (trace.sqrt() * root_scale / norm).to_f64();
            if joint.is_finite() && (joint != 0.0 || trace == zero) {
                result.joint_relative_error = Some(joint);
                result.linearization_warning = joint >= 0.1;
            }
        }
        if projected.is_finite() && projected >= DoubleFloat::from(0.0) {
            let se = projected.sqrt() * root_scale;
            let relative = (se / norm).to_f64();
            let se = se.to_f64();
            if se.is_finite()
                && relative.is_finite()
                && ((se != 0.0 && relative != 0.0) || projected == zero)
            {
                result.radial_standard_error = Some(se);
                result.radial_relative_error = Some(relative);
            }
        }
        if result.radial_standard_error.is_none() {
            result.linearization_warning = true;
        }
        result
    }
}

/// Rare observation-only fallback. Exact native arithmetic on the reported f64
/// entries resolves exponent loss and cancellation before a numerical square
/// root. No evaluator precision, sample, or covariance estimate is changed.
fn exact_projection(
    mut result: ComplexNormUncertainty,
    mean: [f64; 2],
    covariance: [f64; 4],
) -> ComplexNormUncertainty {
    let exact = |v| Rational::try_from(v).expect("validated finite estimate entry");
    let [real, imag] = mean.map(exact);
    let [rr, ii, ri, ir] = covariance.map(exact);
    let squared_norm = &real * &real + &imag * &imag;
    let numerator = &real * &real * &rr + &real * &imag * (ri + ir) + &imag * &imag * &ii;
    let projected = numerator / &squared_norm;
    // A nonzero quantity outside the serialized f64 range is unavailable, not
    // an exact zero error. Native arithmetic retains its wide exponent here.
    let finite_sqrt = |v: &Rational| {
        if *v < 0 {
            return None;
        }
        let n = v.to_multi_prec_float(128).sqrt().to_f64();
        (n.is_finite() && (n != 0.0 || *v == 0)).then_some(n)
    };
    result.joint_relative_error = finite_sqrt(&((rr + ii) / &squared_norm));
    result.linearization_warning = result.joint_relative_error.is_none_or(|v| v >= 0.1);
    result.radial_standard_error = finite_sqrt(&projected);
    result.radial_relative_error = finite_sqrt(&(projected / squared_norm));
    if result.radial_standard_error.is_none() || result.radial_relative_error.is_none() {
        result.linearization_warning = true;
    }
    result
}

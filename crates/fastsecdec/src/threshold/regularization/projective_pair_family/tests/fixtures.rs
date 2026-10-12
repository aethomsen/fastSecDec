//! Test-only proposals for the native massive two-loop three-propagator source.
//! No production discovery or admission routine knows these coefficients.
use super::*;
use symbolica::atom::AtomCore;

pub(crate) fn proposals(
    owner: &PositiveProjectiveQuotient,
    coordinates: &[Symbol],
    limits: &Limits,
) -> Result<Vec<CellProposal>> {
    let c = ResolvedProjectivePairFamily::selected_constant(
        owner,
        ConstantReference {
            cell: 0,
            side: SectionSide::Upper,
        },
        limits,
    )?;
    let a = ResolvedProjectivePairFamily::selected_constant(
        owner,
        ConstantReference {
            cell: 2,
            side: SectionSide::Lower,
        },
        limits,
    )?;
    let b = ResolvedProjectivePairFamily::selected_constant(
        owner,
        ConstantReference {
            cell: 2,
            side: SectionSide::Upper,
        },
        limits,
    )?;
    let d = ResolvedProjectivePairFamily::selected_constant(
        owner,
        ConstantReference {
            cell: 6,
            side: SectionSide::Lower,
        },
        limits,
    )?;
    let u = Atom::var(coordinates[0]);
    let v = Atom::var(coordinates[1]);
    let role = Symbol::parse("pair_fixture::normalized_z", "fastsecdec").map_err(native)?;
    let z = Atom::var(role);
    let basic = |fractions: [Rational; 2], shape| PrefixPiece {
        fractions,
        shape,
        roots: Vec::new(),
        fibre: FibreProposal::Native {
            lower: Atom::zero(),
            upper: None,
            reversed: false,
        },
        rewrites: Vec::new(),
    };
    let full = [Rational::zero(), Rational::one()];
    let mut cells = vec![CellProposal {
        cell: 0,
        constants: Vec::new(),
        pieces: vec![basic(
            full.clone(),
            PrefixShape::Power {
                exponent: 1,
                reversed: false,
            },
        )],
    }];
    let near = |cell: usize, x: Atom, k: Atom, far_first: bool| {
        let aa = Atom::one() + &x;
        let bb = &x * &x - Atom::num(13) * &x + Atom::one();
        let centre = -bb / (Atom::num(2) * &aa);
        let p23 = &x * &x - Atom::num(23) * &x + Atom::one();
        let kk = if far_first {
            (&a - &k) * (&b - &x) * (-p23) / (Atom::num(4) * &aa)
        } else {
            (&k - &b) * (&x - &a) * (-p23) / (Atom::num(4) * &aa)
        };
        let mut rewrites = Vec::new();
        for fibre in 0..2 {
            let y = if fibre == 0 {
                &centre * (Atom::one() - &v)
            } else {
                &centre + &v / (Atom::one() - &v)
            };
            rewrites.push(FactorRewrite {
                fibre,
                term: 0,
                factor: 1,
                expression: &aa * (y - &centre).pow(2) + u.pow(2) * &kk,
            });
        }
        let piece = PrefixPiece {
            fractions: if far_first {
                [Rational::from((1, 2)), Rational::one()]
            } else {
                [Rational::zero(), Rational::from((1, 4))]
            },
            shape: PrefixShape::Power {
                exponent: 2,
                reversed: far_first,
            },
            roots: Vec::new(),
            fibre: FibreProposal::PositiveCentre { centre },
            rewrites,
        };
        let mut pieces = Vec::new();
        if far_first {
            pieces.push(basic(
                [Rational::zero(), Rational::from((1, 2))],
                PrefixShape::Power {
                    exponent: 1,
                    reversed: false,
                },
            ));
            pieces.push(piece);
        } else {
            pieces.push(piece);
            for i in 1..4 {
                pieces.push(basic(
                    [Rational::from((i, 4)), Rational::from((i + 1, 4))],
                    PrefixShape::Power {
                        exponent: 1,
                        reversed: false,
                    },
                ));
            }
        }
        CellProposal {
            cell,
            constants: vec![
                ConstantReference {
                    cell: 2,
                    side: SectionSide::Lower,
                },
                ConstantReference {
                    cell: 2,
                    side: SectionSide::Upper,
                },
            ],
            pieces,
        }
    };
    let k = (&c + &a) / Atom::num(2);
    cells.push(near(1, &a - (&a - &k) * u.pow(2), k, true));
    let l = &b - &a;
    let den = u.pow(2) + (Atom::one() - &u).pow(2);
    let x = &a + &l * u.pow(2) / &den;
    let aa = Atom::one() + &x;
    let centre = -(&x * &x - Atom::num(13) * &x + Atom::one()) / (Atom::num(2) * &aa);
    let scale = &l * &u * (Atom::one() - &u) / (&den * Atom::num(2) * &aa);
    let plus = &centre + &scale * &z;
    let minus = &x / &plus;
    let gap = Atom::num(2) * &scale * &z;
    let equation = z.pow(2) - Atom::num(16) * &x - (&x - &a) * (&b - &x);
    for cell in 2..=4 {
        let (lower, upper, reversed, ff) = match cell {
            2 => (
                Atom::zero(),
                Some(minus.clone()),
                true,
                &aa * &minus * &v * (&gap + &minus * &v),
            ),
            3 => (
                minus.clone(),
                Some(plus.clone()),
                false,
                -&aa * gap.pow(2) * &v * (Atom::one() - &v),
            ),
            _ => (
                plus.clone(),
                None,
                false,
                &aa * &v * ((Atom::one() - &v) * &gap + &v) / (Atom::one() - &v).pow(2),
            ),
        };
        cells.push(CellProposal {
            cell,
            constants: Vec::new(),
            pieces: vec![PrefixPiece {
                fractions: full.clone(),
                shape: PrefixShape::TwoEnded { exponent: 2 },
                roots: vec![RootProposal {
                    role,
                    equation: equation.clone(),
                    bracket: BracketProposal {
                        lower: Rational::one(),
                        upper: Rational::from(12),
                        derivative_margin: Rational::from((1, 1024)),
                    },
                    witness: Rational::from((1, 2)),
                }],
                fibre: FibreProposal::Native {
                    lower,
                    upper,
                    reversed,
                },
                rewrites: vec![FactorRewrite {
                    fibre: 0,
                    term: 0,
                    factor: 1,
                    expression: ff,
                }],
            }],
        });
    }
    let k = &b + (&d - &b) / Atom::num(4);
    cells.push(near(5, &b + (&k - &b) * u.pow(2), k, false));
    cells.push(CellProposal {
        cell: 6,
        constants: Vec::new(),
        pieces: vec![basic(full, PrefixShape::Ray)],
    });
    assert_eq!(cells.len(), 7);
    Ok(cells)
}

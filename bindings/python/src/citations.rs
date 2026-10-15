//! Usage records for Symbolica's existing cumulative bibliography collector.
//!
//! Method references are verified against the primary papers and the geometric
//! cone/triangulation and Taylor-subtraction implementations. They acknowledge
//! mathematical foundations, not a dependency on the authors' software.

use std::sync::atomic::{AtomicBool, Ordering};

use symbolica::api::python::Citation;

static SECTOR_DECOMPOSITION_USED: AtomicBool = AtomicBool::new(false);

/// Called when native generation starts, or a generated kernel artifact loads.
pub(crate) fn mark_generation() {
    SECTOR_DECOMPOSITION_USED.store(true, Ordering::Relaxed);
}

/// Return references for FastSecDec computations used in this process.
/// Importing the module alone does not count as use; querying never resets it.
/// The community host merges these native records with the other HEPKit owners.
pub fn get_citations() -> Vec<Citation> {
    if !SECTOR_DECOMPOSITION_USED.load(Ordering::Relaxed) {
        return Vec::new();
    }
    vec![
        Citation {
            id: "https://github.com/alphal00p/fastSecDec".into(),
            url: "https://github.com/alphal00p/fastSecDec".into(),
            reference: "FastSecDec contributors. FastSecDec: Rust sector decomposition and numerical integration. https://github.com/alphal00p/fastSecDec.".into(),
            bibtex: include_str!("../../../citations/fastsecdec.bib").trim().into(),
            reasons: vec!["Native FastSecDec generation or kernel loading was used.".into()],
            description: "Software repository citation; no publication DOI has been assigned here.".into(),
            relevance: None,
        },
        Citation {
            id: "doi:10.1016/j.cpc.2017.09.015".into(),
            url: "https://doi.org/10.1016/j.cpc.2017.09.015".into(),
            reference: "S. Borowka, G. Heinrich, S. Jahn, S. P. Jones, M. Kerner, J. Schlenk and T. Zirke. pySecDec: a toolbox for the numerical evaluation of multi-scale integrals. Computer Physics Communications 222 (2018), 313. doi:10.1016/j.cpc.2017.09.015.".into(),
            bibtex: include_str!("../../../citations/pysecdec.bib").trim().into(),
            reasons: vec!["Reference sector-decomposition implementation that informed FastSecDec's development and scientific cross-checks.".into()],
            description: "Acknowledgement of pySecDec's scientific and software contribution; FastSecDec implements its own Rust generation pipeline.".into(),
            relevance: None,
        },
        Citation {
            id: "doi:10.1016/j.cpc.2010.04.001".into(),
            url: "https://doi.org/10.1016/j.cpc.2010.04.001".into(),
            reference: "T. Kaneko and T. Ueda. A geometric method of sector decomposition. Computer Physics Communications 181 (2010), 1352–1361. doi:10.1016/j.cpc.2010.04.001.".into(),
            bibtex: include_str!("../../../citations/kaneko-ueda.bib").trim().into(),
            reasons: vec!["Geometric sector decomposition using polynomial exponent supports, normal cones and simplicial monomial maps.".into()],
            description: "Mathematical foundation of FastSecDec's native geometric sector construction.".into(),
            relevance: None,
        },
        Citation {
            id: "doi:10.1016/S0550-3213(00)00429-6".into(),
            url: "https://doi.org/10.1016/S0550-3213(00)00429-6".into(),
            reference: "T. Binoth and G. Heinrich. An automatized algorithm to compute infrared divergent multi-loop integrals. Nuclear Physics B 585 (2000), 741–759. doi:10.1016/S0550-3213(00)00429-6.".into(),
            bibtex: include_str!("../../../citations/binoth-heinrich.bib").trim().into(),
            reasons: vec!["Endpoint Taylor subtraction and Laurent coefficients of dimensionally regulated sector integrals.".into()],
            description: "Methodological foundation for separating endpoint poles from finite parameter integrals.".into(),
            relevance: None,
        },
    ]
}

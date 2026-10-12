// Rebuild: nix-shell -p typst --run 'typst compile docs/contour-deformation.typ docs/benchmark.pdf'
// Measurements are accompanied by native full-covariance summaries and audit records.
#set document(title: "Threshold regularisation in FastSecDec", author: "FastSecDec collaboration")
#set page(paper: "a4", margin: (x: 19mm, top: 18mm, bottom: 19mm),
  footer: context [#text(size: 8pt, fill: rgb("64748b"))[FastSecDec · threshold benchmarks] #h(1fr) #counter(page).display()])
#set text(font: "Libertinus Serif", size: 10.2pt)
#set par(justify: true, leading: 0.54em)
#set heading(numbering: "1.1")
#show heading.where(level: 1): set text(size: 17pt, fill: rgb("173e59"))
#show heading.where(level: 2): set text(size: 11.5pt, fill: rgb("173e59"))
#show table: set par(justify: false)
#set math.equation(numbering: none)
#let note(body) = block(width: 100%, inset: 8pt, fill: rgb("edf4f7"), radius: 3pt, body)

// The opening five overview pages are deliberately unchanged and remain first.
#include "benchmark-overview.typ"
#pagebreak()

#text(size: 25pt, weight: "bold", fill: rgb("173e59"))[Threshold regularisation in FastSecDec]
#v(2mm)
#text(size: 12pt)[Methods, measurements and limits]
#v(2mm)
#text(size: 9pt, fill: rgb("64748b"))[Campaign evidence through 12 October 2026]
#v(4mm)

The opening tables are the primary benchmark comparison: the finite Laurent
coefficient of eight topologies, with QMC and discrete MC shown together.
The chapters that follow explain how to read those measurements, what the
algorithms compute, and which conclusions the available evidence supports.
Complete integral results, local preparation controls and future research
proposals are treated separately throughout.

#note[*Reading guide.* Start with @campaign for column definitions, inputs and
reproduction. @methods defines fixed, dynamic and no-contour prescriptions;
@benchmark-findings interprets their measured accuracy and compatibility.
@implementation explains the principal performance choices. @support-limits
records unfinished algebraic boundaries, and @critical-loci develops the
critical-locus and region-analysis research programme.]

#v(4mm)
#text(size: 14pt, weight: "bold", fill: rgb("173e59"))[Contents]
#v(2mm)
#block[
  #set text(size: 9pt)
  #set par(leading: .4em)
  #outline(title: none, depth: 2, indent: auto)
]

// Chapter styling is scoped after the overview, preserving its rendered pages.
#set heading(supplement: [Chapter])
#show heading.where(level: 1): it => {
  pagebreak(weak: true)
  block(above: 0pt, below: 12pt)[
    #text(size: 9pt, weight: "bold", fill: rgb("64748b"))[CHAPTER #context counter(heading).display("1")]
    #v(2mm)
    #text(size: 21pt, weight: "bold", fill: rgb("173e59"))[#it.body]
  ]
}

#include "report-campaign.typ"
#include "report-methods.typ"
#include "report-results.typ"
#include "report-implementation.typ"
#include "report-support-limits.typ"
#include "report-critical-loci.typ"

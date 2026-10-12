// Opening section. All measurements are supplied by the reviewed campaign data.
// Scoped page/text settings let the parent report resume its normal layout.
#let benchmark-overview() = {
  let data = json("benchmark-overview-results.json")
  assert(data.target_order == 0, message: "Overview requires finite-coefficient data")
  assert(not data.historical_measurements_in_table,
    message: "Historical measurements must not enter the new campaign table")
  set page(paper: "a4", flipped: true, margin: (x: 10mm, y: 10mm))
  set text(font: "Libertinus Sans", size: 7.5pt, fill: rgb("172b42"))
  set par(leading: .35em)
  let ink = rgb("183b56")
  let rule = .35pt + rgb("c7d4df")
  let missing = text(fill: rgb("82919f"))[—]
  let show-value(value) = if value == none { missing } else { str(value) }
  let cell(value, ..args) = table.cell(..args, align: center + horizon, show-value(value))
  let columns = (23fr, 12fr, 16fr, 16fr, 15fr, 12fr, 24fr, 17fr, 29fr, 24fr, 16fr, 20fr, 22fr, 26fr)
  let header = table.header(
    [*Prescription*], [*Sampler*],
    [*Generation*\ s], [*Gen. RAM*\ GB], [*Artifacts*\ MB], [*Sectors*],
    [*f64 max / avg*\ µs / sample], [*Int. RAM*\ GB],
    [*Σ f64 / checked*\ µs / sweep],
    [*Average work*\ rel. σ(|I|)\*], [*C*], [*max / |I|*],
    [*Sector calls*\ ~600 s · million],
    [*Rel. σ(|I|)*\ *600 s*\*],
  )
  let topology-block(topology) = {
    let cells = ()
    for method in topology.methods {
      for (index, sampler) in method.samplers.enumerate() {
        if index == 0 {
          cells.push(table.cell(rowspan: 2, align: left + horizon,
            text(weight: "semibold")[#method.label]))
        }
        let sampler-color = if sampler.id == "qmc" { rgb("e4f2f4") } else { rgb("fff0dd") }
        cells.push(table.cell(fill: sampler-color, align: center + horizon,
          text(weight: "semibold")[#sampler.label]))
        if index == 0 {
          for key in ("generation_seconds", "generation_peak_gb", "artifact_mb", "stochastic_sectors") {
            let value = method.generation.at(key)
            if key == "generation_seconds" and method.generation_status == "pending" {
              value = "Pending"
            }
            if key == "generation_seconds" and method.id == "no-contour" {
              if method.generation_status == "refused" and method.status_detail.contains("unsupported threshold generation") {
                value = "Unsupported"
              } else if method.generation_status == "not_available" {
                value = "In preparation"
              } else if method.generation_status == "refused" {
                value = "In development"
              }
            }
            cells.push(cell(value, rowspan: 2))
          }
          cells.push(cell(method.kernel_cost.f64_max_avg_us, rowspan: 2))
        }
        cells.push(cell(sampler.metrics.integration_peak_gb))
        if index == 0 {
          cells.push(cell(method.kernel_cost.sum_f64_checked_us, rowspan: 2))
        }
        let normalized = sampler.at("normalized_metrics", default: (:))
        cells.push(cell(normalized.at("work_radial_relative_se", default: none)))
        let weights = sampler.at("weight_metrics", default: (:))
        cells.push(cell(weights.at("display_cancellation_ratio", default: weights.at("absolute_weight_integral_over_norm", default: none))))
        cells.push(cell(weights.at("display_maximum_ratio", default: weights.at("maximum_observed_weight_over_norm", default: none))))
        cells.push(cell(normalized.at("time_reported_sector_calls_million", default: none)))
        cells.push(table.cell(fill: rgb("e3f0e7"), align: center + horizon,
          text(weight: "bold")[#show-value(normalized.at("time_radial_relative_se", default: none))]))
      }
    }
    block(breakable: false, width: 100%)[
      #block(fill: ink, inset: (x: 6pt, y: 4pt), width: 100%)[
        #text(fill: white, weight: "bold", size: 9pt)[#topology.label]
        #h(1fr)
        #text(fill: rgb("d8e7ef"), size: 7.5pt)[#topology.point]
        #let target = topology.at("display_work_target_average_per_sector", default: none)
        #if target != none { [#h(6pt) #text(fill: white, size: 7pt)[· average work target #calc.round(target / 1000000, digits: 3)M / sector]] }
      ]
      #table(columns: columns, inset: (x: 3pt, y: 4.2pt), stroke: rule,
        align: center + horizon,
        fill: (x, y) => if y == 0 { rgb("eef3f7") },
        header, ..cells,
      )
    ]
  }
  let groups = ((0, 1, 2), (3, 7), (4,), (5, 6))
  for (page-index, group) in groups.enumerate() {
    if page-index > 0 { pagebreak() }
    text(size: 19pt, weight: "bold", fill: ink)[Overview]
    h(1fr)
    text(size: 9pt, fill: rgb("456078"))[Finite coefficient $epsilon^0$ · 25-core integration]
    linebreak()
    text(size: 7pt, fill: rgb("456078"))[\* MC-equivalent normalization · ≥ observed calls · † first-order norm propagation may be unreliable]
    v(5mm)
    for index in group {
      topology-block(data.topologies.at(index))
      v(4mm)
    }
  }
}

#benchmark-overview()

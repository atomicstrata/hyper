# Run in a Julia environment with SimpleHypergraphs.jl installed.
# API: https://github.com/pszufe/SimpleHypergraphs.jl/blob/master/src/io_hif.jl
using SimpleHypergraphs

h = Hypergraph{Int}(3, 2) # vertex 3 is isolated; edge 2 is empty
h[1, 1] = 1
h[2, 1] = 1
hg_save("julia.hif.json", h; format=HIF_Format(), pretty=true)
restored = hg_load("julia.hif.json"; format=HIF_Format(), T=Int)
@assert nhv(restored) == 3
@assert nhe(restored) == 2
println("Exported julia.hif.json; load with hyper_viz.HifDocument.load from Python.")
# This exporter includes incidence weights, even for weight 1. Hyper preserves
# them in full-document interchange and rejects viewer conversion explicitly.

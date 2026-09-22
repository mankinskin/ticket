# Interview: Dependency Graph Rendering

**Date:** 2026-04-08
**Applies to:** `8d861d64` (DOM Element Graph), `12d3c38b` (GPU 3D graph)

## Question

Should the SVG graph be kept as a fallback view? Should the dependency graph support zoom/pan?

## Answer

**Remove the SVG dependency graph entirely. Use the DOM Element Graph as it is currently implemented in the log-viewer HypergraphView — the most mature and detailed graph rendering pipeline. SVG graphs should be part of inline graph rendering in markdown documents, treated as glyphs for machine-readable visual proofs.**

## Implications

- `5711c397` (SVG dependency graph) cancelled and replaced by `8d861d64`
- Reference implementation: log-viewer HypergraphView pipeline
- Graph nodes are actual DOM elements (HTML/CSS) — not drawn SVG shapes
- Edges drawn as canvas/WebGPU lines between DOM element positions
- GPU compositing layer is fundamental (not optional) to this architecture
- SVG graphs are a separate concept: inline markdown rendering for proofs/diagrams
- When implementing SVG inline in Markdown, keep it entirely separate from the dependency graph pipeline

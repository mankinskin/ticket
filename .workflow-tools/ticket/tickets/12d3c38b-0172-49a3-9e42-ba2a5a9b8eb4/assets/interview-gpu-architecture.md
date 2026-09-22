# Interview: GPU Features Priority and Architecture

**Date:** 2026-04-08
**Applies to:** `12d3c38b` (GPU 3D graph), `dbd048a0` (WgpuOverlay), `512986e0` (Theme settings), Epic `35a6d14b`

## Question

The existing viewer has GPU-accelerated 3D dependency graph + WgpuOverlay (particles, CRT shader, vignette). Track 5 is currently marked low priority.

- Should GPU features be deferred entirely until after all other tracks are complete?
- Or is the 3D dependency graph a high-visibility feature you want earlier?
- Is WgpuOverlay (purely cosmetic effects) still wanted, or can it be dropped?

## Answer

**High priority. Ideally all content is rendered through a GPU-accelerated canvas. We want to draw the native HTML and CSS flexbox layout to GPU buffers and draw a full-screen canvas background behind the interactive HTML elements. This has been most completely implemented in log-viewer.**

## Implications

- **Track 5 priority escalated from Low to High/Critical** — GPU rendering is core architecture, not optional polish
- The entire UI should render through a GPU-accelerated canvas as a compositor layer
- Architecture: HTML/CSS flexbox layout → captured to GPU buffers → full-screen WebGPU canvas background → interactive HTML elements overlaid on top
- This is the "glass panel" pattern from context-editor, but taken to full-screen compositing
- log-viewer has the most complete reference implementation — study its GPU pipeline
- WgpuOverlay is NOT cosmetic — it's the core rendering layer
- The SVG dependency graph (Track 2, `5711c397`) should be designed as a stepping stone toward the GPU graph, not an alternative
- This may affect the scaffold tickets — crate setup should include WebGPU canvas from the start
- Research needed: log-viewer's GPU buffer extraction, DOM-to-GPU pipeline, canvas compositing approach

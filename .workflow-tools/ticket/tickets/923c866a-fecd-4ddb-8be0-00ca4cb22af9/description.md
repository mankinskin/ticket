# Goal

Refine graph node selection and focus falloff so the selected ticket stays emphasized, linked context remains visible, and clicking outside the graph clears selection.

# Scope

- define a stepwise focus-strength model based on path distance from the selected node
- make low-focus transparency apply only to nodes outside the selected or linked focus neighborhood
- preserve direct neighbors as the default emphasized neighborhood while allowing graded wider-context falloff
- apply selected-state effects for scale, glow or halo, edge emphasis, and neighborhood tint
- clear node selection when the user clicks or taps outside any graph node
- add deterministic browser and Playwright validation hooks for focus effects and deselection

# Acceptance

- only nodes outside the active focus neighborhood render with low-focus transparency
- selecting a node keeps the selected node and direct linked neighbors visually emphasized by default
- the renderer exposes explicit stepwise focus bands by path distance for broader neighborhood presentation
- selected-state scale, glow or halo, edge highlight, and neighborhood tint are observable through stable DOM/render hooks
- clicking or tapping on empty graph space clears the active selection
- release Playwright coverage proves deselection and neighborhood emphasis behavior

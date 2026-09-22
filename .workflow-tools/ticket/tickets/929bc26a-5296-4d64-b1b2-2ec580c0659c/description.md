# Goal

Keep graph content behind sidebar and viewport panels while using those panel bounds to bias graph framing, focus centering, and node placement.

# Scope

- enforce projection behind the UI screen plane so graph nodes never render above app panels
- compute the remaining visible graph area after panel occupancy and bias camera focus toward that area
- add panel-edge avoidance or repulsion so nodes do not cluster directly beneath important panel chrome
- preserve natural interaction between graph motion and panel-covered regions without visually crossing the panel plane
- add release Playwright and browser checks that exercise changing panel states and verify layering and framing

# Acceptance

- graph nodes remain visually behind sidebar and in-viewport panels at all times
- camera focus centers within or toward the remaining visible graph region outside active panels
- dense node clusters are biased away from panel edges instead of sitting directly under the UI chrome
- the graph still supports smooth focus changes and layout updates while respecting panel occupancy
- release Playwright coverage proves panel-safe layering, framing shifts, and panel-edge avoidance under changing panel states

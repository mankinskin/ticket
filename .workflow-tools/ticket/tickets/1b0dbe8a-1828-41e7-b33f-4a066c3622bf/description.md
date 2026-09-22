W4. List immediate parents and children in the content panel; make them clickable to jump the selection to the linked node; the rendered graph must update from cache instead of fetching at click time.

Requirements:
- Render immediate parents + children in the (extended) content panel.
- Clicking a neighbour retargets selection and focused neighbourhood.
- Graph retarget is served from the client-side cache (from W2), never an on-click refetch.

Depends on W1 (extended content panel) and W2 (client-side graph cache). Spec 8c4d51ef.
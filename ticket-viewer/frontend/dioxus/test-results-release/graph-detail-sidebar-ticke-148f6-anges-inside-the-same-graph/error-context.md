# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: graph-detail-sidebar.spec.ts >> ticket-viewer — graph selection updates right detail sidebar >> dragged graph layout and camera zoom persist when focus changes inside the same graph
- Location: e2e-release\graph-detail-sidebar.spec.ts:834:7

# Error details

```
Error: same-graph focus changes should preserve the dragged horizontal node offset

expect(received).toBeLessThan(expected)

Expected: < 24
Received:   34.199951171875
```

# Page snapshot

```yaml
- generic [ref=e5]:
  - banner [ref=e6]:
    - generic [ref=e7]:
      - button "Collapse tickets sidebar" [ref=e8] [cursor=pointer]:
        - img [ref=e9]
      - generic [ref=e10]: 🎫
      - generic [ref=e11]: e2e-release-store
      - generic [ref=e12]: e2e-release-store--344e8f42
    - button "Theme settings" [ref=e15] [cursor=pointer]:
      - img [ref=e16]
  - generic [ref=e18]:
    - generic [ref=e19]:
      - generic [ref=e20]:
        - heading "Tickets" [level=2] [ref=e21]
        - generic [ref=e22]: "1"
        - button "Collapse sidebar" [ref=e23] [cursor=pointer]:
          - img [ref=e25]
      - generic [ref=e28]:
        - generic [ref=e29]:
          - button "Browse" [pressed] [ref=e30] [cursor=pointer]
          - button "Next" [ref=e31] [cursor=pointer]
          - button "Blockers" [ref=e32] [cursor=pointer]
          - button "Unblocked" [ref=e33] [cursor=pointer]
        - generic [ref=e34]:
          - generic [ref=e35]:
            - textbox "Search titles/descriptions or use id:, title:, state:, type:" [ref=e36]: id:b3a60766-ec18-41c1-8430-dcd108165c5e
            - generic [ref=e37]: "Free text searches titles, descriptions, and ticket IDs (including partial-UUID substrings). Patterns: id:<value>, title:<value>, state:<value>/status:<value>, type:<value>/ticket_type:<value>. Terms are ANDed; quote phrases."
          - generic [ref=e38]:
            - button "All" [pressed] [ref=e39] [cursor=pointer]
            - button "planning" [ref=e40] [cursor=pointer]
            - button "ready" [ref=e41] [cursor=pointer]
            - button "impl" [ref=e42] [cursor=pointer]
            - button "review" [ref=e43] [cursor=pointer]
            - button "on-hold" [ref=e44] [cursor=pointer]
            - button "done" [ref=e45] [cursor=pointer]
            - button "cancelled" [ref=e46] [cursor=pointer]
            - button "Toggle batch selection" [ref=e47] [cursor=pointer]: ☑
            - button "Create new ticket" [ref=e48] [cursor=pointer]: + New
          - generic [ref=e52]:
            - button "Expand ticket files" [ref=e53] [cursor=pointer]: ▸
            - button "Release E2E graph root planning" [ref=e54] [cursor=pointer]:
              - generic [ref=e56]: Release E2E graph root
              - generic [ref=e57]: planning
      - separator "Resize panel" [ref=e58]
    - generic [ref=e59]:
      - generic [ref=e60]:
        - generic [ref=e61]: "View:"
        - button "Graph" [ref=e62] [cursor=pointer]
        - button "Split" [ref=e63] [cursor=pointer]
        - button "Content" [ref=e64] [cursor=pointer]
      - generic [ref=e67]:
        - img
        - generic:
          - generic:
            - generic [ref=e69] [cursor=pointer]:
              - generic [ref=e71]: planning
              - generic [ref=e72]: Release E2E graph root
              - generic [ref=e73]:
                - generic [ref=e74]: Ticket
                - generic [ref=e75]: b3a60766
            - generic [ref=e77] [cursor=pointer]:
              - generic [ref=e79]: planning
              - generic [ref=e80]: Release E2E search fixture epsilon
              - generic [ref=e81]:
                - generic [ref=e82]: Ticket
                - generic [ref=e83]: 0e7da84e
            - generic [ref=e85] [cursor=pointer]:
              - generic [ref=e87]: planning
              - generic [ref=e88]: Release E2E search fixture delta
              - generic [ref=e89]:
                - generic [ref=e90]: Ticket
                - generic [ref=e91]: 5ea85741
            - generic [ref=e93] [cursor=pointer]:
              - generic [ref=e95]: planning
              - generic [ref=e96]: Release E2E search fixture alpha
              - generic [ref=e97]:
                - generic [ref=e98]: Ticket
                - generic [ref=e99]: f32e9df7
            - generic [ref=e101] [cursor=pointer]:
              - generic [ref=e103]: planning
              - generic [ref=e104]: Release E2E navigation fixture
              - generic [ref=e105]:
                - generic [ref=e106]: Ticket
                - generic [ref=e107]: e7a9252f
            - generic [ref=e109] [cursor=pointer]:
              - generic [ref=e111]: planning
              - generic [ref=e112]: Release E2E search fixture gamma
              - generic [ref=e113]:
                - generic [ref=e114]: Ticket
                - generic [ref=e115]: 0b4337f6
            - generic [ref=e117] [cursor=pointer]:
              - generic [ref=e119]: planning
              - generic [ref=e120]: Release E2E search fixture beta
              - generic [ref=e121]:
                - generic [ref=e122]: Ticket
                - generic [ref=e123]: cbf4d945
            - generic [ref=e125] [cursor=pointer]:
              - generic [ref=e127]: planning
              - generic [ref=e128]: Release E2E legacy description fixture
              - generic [ref=e129]:
                - generic [ref=e130]: Ticket
                - generic [ref=e131]: 3a1ec9f8
            - generic [ref=e133] [cursor=pointer]:
              - generic [ref=e135]: ready
              - generic [ref=e136]: Release E2E ready prerequisite
              - generic [ref=e137]:
                - generic [ref=e138]: Ticket
                - generic [ref=e139]: 070c8972
            - generic [ref=e141] [cursor=pointer]:
              - generic [ref=e143]: done
              - generic [ref=e144]: Release E2E completed prerequisite
              - generic [ref=e145]:
                - generic [ref=e146]: Ticket
                - generic [ref=e147]: e108982f
            - generic [ref=e149] [cursor=pointer]:
              - generic [ref=e151]: in-implementation
              - generic [ref=e152]: Release E2E implementation prerequisite
              - generic [ref=e153]:
                - generic [ref=e154]: Ticket
                - generic [ref=e155]: 4dfa31b5
            - generic [ref=e157] [cursor=pointer]:
              - generic [ref=e159]: in-review
              - generic [ref=e160]: Release E2E review prerequisite
              - generic [ref=e161]:
                - generic [ref=e162]: Ticket
                - generic [ref=e163]: 68e2a319
          - generic: "Left-drag: orbit · Right-drag: pan · Scroll: zoom · Click card: open"
          - generic: 12 nodes
        - button "⚙" [ref=e165] [cursor=pointer]
```

# Test source

```ts
  804  |       selectedLod: 'rich',
  805  |       visibleNodeCount: expect.any(Number),
  806  |     });
  807  | 
  808  |     const childMetrics = await graphLodMetrics(page, candidate.childId);
  809  |     expect(childMetrics, 'selected child node should expose LOD metrics').not.toBeNull();
  810  |     expect(
  811  |       childMetrics!.collapsedNodes,
  812  |       'after selection, other visible nodes should still use smaller compact or minimal tiers',
  813  |     ).toBeGreaterThan(0);
  814  | 
  815  |     await zoomGraph(page, -480, 4);
  816  | 
  817  |     await expect.poll(() => graphLodMetrics(page, candidate.childId), {
  818  |       timeout: 20_000,
  819  |     }).toMatchObject({
  820  |       selectedLod: 'rich',
  821  |       visibleNodeCount: expect.any(Number),
  822  |     });
  823  | 
  824  |     const zoomedInMetrics = await graphLodMetrics(page, candidate.childId);
  825  |     expect(zoomedInMetrics, 'zoomed-in child graph node should expose LOD metrics').not.toBeNull();
  826  |     expect(
  827  |       zoomedInMetrics!.minimalNodes,
  828  |       'zooming back in should not increase the number of minimal visible nodes',
  829  |     ).toBeLessThanOrEqual(zoomedOutMetrics!.minimalNodes);
  830  | 
  831  |     await attachScreenshot(page, testInfo, 'graph-node-lod-tiers');
  832  |   });
  833  | 
  834  |   test('dragged graph layout and camera zoom persist when focus changes inside the same graph', async ({ page }, testInfo) => {
  835  |     test.setTimeout(120_000);
  836  | 
  837  |     const candidate = await findGraphSelectionCandidate(page);
  838  | 
  839  |     await openCandidateTicket(page, candidate);
  840  | 
  841  |     await page.getByRole('button', { name: /^Graph$/ }).first().click();
  842  |     await expect(page.locator('#graph3d-container')).toBeVisible({ timeout: 30_000 });
  843  | 
  844  |     await expect.poll(() => graphNodeMetrics(page, candidate.rootId), {
  845  |       timeout: 30_000,
  846  |     }).not.toBeNull();
  847  |     await expect.poll(() => graphNodeMetrics(page, candidate.childId), {
  848  |       timeout: 30_000,
  849  |     }).not.toBeNull();
  850  | 
  851  |     const childBeforeDrag = await graphNodeMetrics(page, candidate.childId);
  852  |     expect(childBeforeDrag, 'child graph node should expose drag metrics before movement').not.toBeNull();
  853  | 
  854  |     await dragGraphNode(page, candidate.childId, 120, 60);
  855  | 
  856  |     await expect.poll(async () => {
  857  |       const metrics = await graphNodeMetrics(page, candidate.childId);
  858  |       if (!metrics || !childBeforeDrag) {
  859  |         return 0;
  860  |       }
  861  |       return Math.hypot(
  862  |         metrics.centerX - childBeforeDrag.centerX,
  863  |         metrics.centerY - childBeforeDrag.centerY,
  864  |       );
  865  |     }, {
  866  |       timeout: 20_000,
  867  |     }).toBeGreaterThan(40);
  868  | 
  869  |     await zoomGraph(page, 480, 4);
  870  | 
  871  |     const rootBeforeFocus = await graphNodeMetrics(page, candidate.rootId);
  872  |     const childBeforeFocus = await graphNodeMetrics(page, candidate.childId);
  873  |     const cameraDistanceBeforeFocus = await graphCameraDistance(page);
  874  |     expect(rootBeforeFocus, 'root graph node should remain visible after dragging and zooming').not.toBeNull();
  875  |     expect(childBeforeFocus, 'dragged child graph node should remain visible after zooming').not.toBeNull();
  876  |     expect(cameraDistanceBeforeFocus, 'graph container should expose current camera distance').not.toBeNull();
  877  | 
  878  |     const childNode = page.locator(`#graph3d-container [data-node-id="${candidate.childId}"]`).first();
  879  |     await expect(childNode).toBeVisible();
  880  |     await childNode.click();
  881  | 
  882  |     await expect.poll(() => graphLodMetrics(page, candidate.childId), {
  883  |       timeout: 20_000,
  884  |     }).toMatchObject({
  885  |       selectedLod: 'rich',
  886  |       visibleNodeCount: expect.any(Number),
  887  |     });
  888  | 
  889  |     const rootAfterFocus = await graphNodeMetrics(page, candidate.rootId);
  890  |     const childAfterFocus = await graphNodeMetrics(page, candidate.childId);
  891  |     const cameraDistanceAfterFocus = await graphCameraDistance(page);
  892  |     expect(rootAfterFocus, 'root node should remain visible after focus change').not.toBeNull();
  893  |     expect(childAfterFocus, 'dragged child node should remain visible after focus change').not.toBeNull();
  894  |     expect(cameraDistanceAfterFocus, 'camera distance should still be exposed after focus change').not.toBeNull();
  895  | 
  896  |     const deltaBeforeFocusX = childBeforeFocus!.centerX - rootBeforeFocus!.centerX;
  897  |     const deltaBeforeFocusY = childBeforeFocus!.centerY - rootBeforeFocus!.centerY;
  898  |     const deltaAfterFocusX = childAfterFocus!.centerX - rootAfterFocus!.centerX;
  899  |     const deltaAfterFocusY = childAfterFocus!.centerY - rootAfterFocus!.centerY;
  900  | 
  901  |     expect(
  902  |       Math.abs(deltaAfterFocusX - deltaBeforeFocusX),
  903  |       'same-graph focus changes should preserve the dragged horizontal node offset',
> 904  |     ).toBeLessThan(24);
       |       ^ Error: same-graph focus changes should preserve the dragged horizontal node offset
  905  |     expect(
  906  |       Math.abs(deltaAfterFocusY - deltaBeforeFocusY),
  907  |       'same-graph focus changes should preserve the dragged vertical node offset',
  908  |     ).toBeLessThan(24);
  909  |     expect(
  910  |       Math.abs(cameraDistanceAfterFocus! - cameraDistanceBeforeFocus!),
  911  |       'focus changes should preserve the user-adjusted camera zoom distance',
  912  |     ).toBeLessThan(0.25);
  913  | 
  914  |     await attachScreenshot(page, testInfo, 'graph-persisted-layout-and-zoom');
  915  |   });
  916  | 
  917  |   test('graph mode defaults to hierarchical isometric controls and preserves top-to-bottom depth ordering', async ({ page }, testInfo) => {
  918  |     test.setTimeout(120_000);
  919  | 
  920  |     const candidate = await findGraphSelectionCandidate(page);
  921  | 
  922  |     await openCandidateTicket(page, candidate);
  923  | 
  924  |     await page.getByRole('button', { name: /^Graph$/ }).first().click();
  925  |     await expect(page.locator('#graph3d-container')).toBeVisible({ timeout: 30_000 });
  926  | 
  927  |     await page.waitForFunction(
  928  |       ({ rootTitle, childTitle }) => {
  929  |         const root = document.getElementById('graph3d-container');
  930  |         if (!root) {
  931  |           return false;
  932  |         }
  933  |         const text = Array.from(root.querySelectorAll('[data-node-idx]'))
  934  |           .filter((node) => (node as HTMLElement).style.display !== 'none')
  935  |           .map((node) => node.textContent || '');
  936  |         return text.some((value) => value.includes(rootTitle)) &&
  937  |           text.some((value) => value.includes(childTitle));
  938  |       },
  939  |       { rootTitle: candidate.rootTitle, childTitle: candidate.childTitle },
  940  |       { timeout: 30_000 },
  941  |     );
  942  | 
  943  |     const rootNode = page
  944  |       .locator('#graph3d-container [data-node-idx]:visible')
  945  |       .filter({ hasText: candidate.rootTitle })
  946  |       .first();
  947  |     const childNode = page
  948  |       .locator('#graph3d-container [data-node-idx]:visible')
  949  |       .filter({ hasText: candidate.childTitle })
  950  |       .first();
  951  | 
  952  |     await expect(rootNode).toBeVisible();
  953  |     await expect(childNode).toBeVisible();
  954  | 
  955  |     const rootBox = await rootNode.boundingBox();
  956  |     const childBox = await childNode.boundingBox();
  957  |     expect(rootBox, 'root graph node must expose a screen box').not.toBeNull();
  958  |     expect(childBox, 'child graph node must expose a screen box').not.toBeNull();
  959  |     expect(
  960  |       childBox!.y,
  961  |       'child node should render below the root in the default hierarchy view',
  962  |     ).toBeGreaterThan(rootBox!.y + 10);
  963  | 
  964  |     await page.getByTitle('Graph settings').click();
  965  |     const hierarchicalButton = page.getByRole('button', { name: 'Hierarchical 3D' });
  966  |     const orthographicButton = page.getByRole('button', { name: 'Orthographic' });
  967  |     await expect(hierarchicalButton).toBeVisible();
  968  |     await expect(orthographicButton).toBeVisible();
  969  |     await expect(hierarchicalButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);
  970  |     await expect(orthographicButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);
  971  | 
  972  |     await attachScreenshot(page, testInfo, 'graph-default-isometric-layout');
  973  |   });
  974  | 
  975  |   test('graph settings can switch away from and restore the default layout and projection', async ({ page }, testInfo) => {
  976  |     test.setTimeout(120_000);
  977  | 
  978  |     const candidate = await findGraphSelectionCandidate(page);
  979  | 
  980  |     await openCandidateTicket(page, candidate);
  981  | 
  982  |     await page.getByRole('button', { name: /^Graph$/ }).first().click();
  983  |     await expect(page.locator('#graph3d-container')).toBeVisible({ timeout: 30_000 });
  984  | 
  985  |     await expect.poll(() => graphHierarchyMetrics(page, candidate.rootId, candidate.childId), {
  986  |       timeout: 30_000,
  987  |     }).toMatchObject({
  988  |       visibleNodeCount: expect.any(Number),
  989  |     });
  990  | 
  991  |     const defaultMetrics = await graphHierarchyMetrics(page, candidate.rootId, candidate.childId);
  992  |     expect(defaultMetrics, 'default graph layout should expose root and child positions').not.toBeNull();
  993  |     expect(
  994  |       defaultMetrics!.childY,
  995  |       'default hierarchical view should render the child below the root',
  996  |     ).toBeGreaterThan(defaultMetrics!.rootY + 10);
  997  | 
  998  |     await page.getByTitle('Graph settings').click();
  999  | 
  1000 |     const hierarchicalButton = page.getByRole('button', { name: 'Hierarchical 3D' });
  1001 |     const flatButton = page.getByRole('button', { name: 'Flat 2D' });
  1002 |     const perspectiveButton = page.getByRole('button', { name: 'Perspective' });
  1003 |     const orthographicButton = page.getByRole('button', { name: 'Orthographic' });
  1004 | 
```
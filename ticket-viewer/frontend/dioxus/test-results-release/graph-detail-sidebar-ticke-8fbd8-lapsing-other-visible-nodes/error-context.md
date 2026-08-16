# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: graph-detail-sidebar.spec.ts >> ticket-viewer — graph selection updates right detail sidebar >> graph node LOD keeps the active selection rich while collapsing other visible nodes
- Location: e2e-release\graph-detail-sidebar.spec.ts:749:7

# Error details

```
Error: workspace graph should collapse at least one non-selected visible node to a smaller LOD tier

expect(received).toBeGreaterThan(expected)

Expected: > 0
Received:   0
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
      - generic [ref=e12]: e2e-release-store--a5d633fb
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
            - textbox "Search titles/descriptions or use id:, title:, state:, type:" [ref=e36]: id:c5e001f1-dd1f-4bd0-a334-213fc17e1fdb
            - generic [ref=e37]: "Free text searches titles, descriptions, and ticket IDs (including partial-UUID substrings). Patterns: id:<value>, title:<value>, state:<value>/status:<value>, type:<value>/ticket_type:<value>. Terms are ANDed; quote phrases."
          - generic [ref=e38]:
            - button "All" [pressed] [ref=e39] [cursor=pointer]
            - button "open" [ref=e40] [cursor=pointer]
            - button "planned" [ref=e41] [cursor=pointer]
            - button "impl" [ref=e42] [cursor=pointer]
            - button "review" [ref=e43] [cursor=pointer]
            - button "done" [ref=e44] [cursor=pointer]
            - button "cancelled" [ref=e45] [cursor=pointer]
            - button "Toggle batch selection" [ref=e46] [cursor=pointer]: ☑
            - button "Create new ticket" [ref=e47] [cursor=pointer]: + New
          - generic [ref=e51]:
            - button "Expand ticket files" [ref=e52] [cursor=pointer]: ▸
            - button "Release E2E graph root open" [ref=e53] [cursor=pointer]:
              - generic [ref=e55]: Release E2E graph root
              - generic [ref=e56]: open
      - separator "Resize panel" [ref=e57]
    - generic [ref=e58]:
      - generic [ref=e59]:
        - generic [ref=e60]: "View:"
        - button "Graph" [active] [ref=e61] [cursor=pointer]
        - button "Split" [ref=e62] [cursor=pointer]
        - button "Content" [ref=e63] [cursor=pointer]
      - generic [ref=e66]:
        - img
        - generic:
          - generic:
            - generic [ref=e68] [cursor=pointer]:
              - generic [ref=e70]: open
              - generic [ref=e71]: Release E2E navigation fixture
              - generic [ref=e72]:
                - generic [ref=e73]: Ticket
                - generic [ref=e74]: 189a8dc5
            - generic [ref=e76] [cursor=pointer]:
              - generic [ref=e78]: open
              - generic [ref=e79]: Release E2E search fixture beta
              - generic [ref=e80]:
                - generic [ref=e81]: Ticket
                - generic [ref=e82]: 7995a4bf
            - generic [ref=e84] [cursor=pointer]:
              - generic [ref=e86]: open
              - generic [ref=e87]: Release E2E graph root
              - generic [ref=e88]:
                - generic [ref=e89]: Ticket
                - generic [ref=e90]: c5e001f1
            - generic [ref=e92] [cursor=pointer]:
              - generic [ref=e94]: open
              - generic [ref=e95]: Release E2E legacy description fixture
              - generic [ref=e96]:
                - generic [ref=e97]: Ticket
                - generic [ref=e98]: 3a1ec9f8
            - generic [ref=e100] [cursor=pointer]:
              - generic [ref=e102]: open
              - generic [ref=e103]: Release E2E search fixture epsilon
              - generic [ref=e104]:
                - generic [ref=e105]: Ticket
                - generic [ref=e106]: 42cd09bf
            - generic [ref=e108] [cursor=pointer]:
              - generic [ref=e110]: open
              - generic [ref=e111]: Release E2E search fixture delta
              - generic [ref=e112]:
                - generic [ref=e113]: Ticket
                - generic [ref=e114]: 4a8bac34
            - generic [ref=e116] [cursor=pointer]:
              - generic [ref=e118]: open
              - generic [ref=e119]: Release E2E search fixture gamma
              - generic [ref=e120]:
                - generic [ref=e121]: Ticket
                - generic [ref=e122]: "1e891896"
            - generic [ref=e124] [cursor=pointer]:
              - generic [ref=e126]: open
              - generic [ref=e127]: Release E2E search fixture alpha
              - generic [ref=e128]:
                - generic [ref=e129]: Ticket
                - generic [ref=e130]: d15253b2
            - generic [ref=e132] [cursor=pointer]:
              - generic [ref=e134]: planned
              - generic [ref=e135]: Release E2E ready prerequisite
              - generic [ref=e136]:
                - generic [ref=e137]: Ticket
                - generic [ref=e138]: 1ee5dc7b
            - generic [ref=e140] [cursor=pointer]:
              - generic [ref=e142]: in-implementation
              - generic [ref=e143]: Release E2E implementation prerequisite
              - generic [ref=e144]:
                - generic [ref=e145]: Ticket
                - generic [ref=e146]: 5419662d
            - generic [ref=e148] [cursor=pointer]:
              - generic [ref=e150]: in-review
              - generic [ref=e151]: Release E2E review prerequisite
              - generic [ref=e152]:
                - generic [ref=e153]: Ticket
                - generic [ref=e154]: e5d14cdc
            - generic [ref=e156] [cursor=pointer]:
              - generic [ref=e158]: done
              - generic [ref=e159]: Release E2E completed prerequisite
              - generic [ref=e160]:
                - generic [ref=e161]: Ticket
                - generic [ref=e162]: 32f99a29
          - generic: "Left-drag: orbit · Right-drag: pan · Scroll: zoom · Click card: open"
          - generic: 12 nodes
        - button "⚙" [ref=e164] [cursor=pointer]
```

# Test source

```ts
  691 |         return Number.parseFloat(getComputedStyle(node).opacity || '1') < 0.5;
  692 |       }).length;
  693 |       const childDistance = Math.hypot(childCx - containerCx, childCy - containerCy);
  694 | 
  695 |       return childDistance <= maxDistance && dimmedCount > 0;
  696 |     }, {
  697 |       childId: candidate.childId,
  698 |       maxDistance: Math.max(48, initialDistance! * 0.75),
  699 |     }), {
  700 |       timeout: 20_000,
  701 |     }).toBe(true);
  702 | 
  703 |     const focusedMetrics = await page.evaluate((childId) => {
  704 |       const container = document.getElementById('graph3d-container');
  705 |       if (!container) {
  706 |         return null;
  707 |       }
  708 |       const visibleNodes = Array.from(container.querySelectorAll('[data-node-id]')).filter((node) => {
  709 |         const element = node as HTMLElement;
  710 |         return element.style.display !== 'none';
  711 |       }) as HTMLElement[];
  712 |       const child = visibleNodes.find((node) => node.dataset.nodeId === childId);
  713 |       if (!child) {
  714 |         return null;
  715 |       }
  716 | 
  717 |       const containerRect = container.getBoundingClientRect();
  718 |       const childRect = child.getBoundingClientRect();
  719 |       const containerCx = containerRect.left + containerRect.width / 2;
  720 |       const containerCy = containerRect.top + containerRect.height / 2;
  721 |       const childCx = childRect.left + childRect.width / 2;
  722 |       const childCy = childRect.top + childRect.height / 2;
  723 |       const dimmedCount = visibleNodes.filter((node) => {
  724 |         if (node.dataset.nodeId === childId) {
  725 |           return false;
  726 |         }
  727 |         return Number.parseFloat(getComputedStyle(node).opacity || '1') < 0.5;
  728 |       }).length;
  729 | 
  730 |       return {
  731 |         childDistance: Math.hypot(childCx - containerCx, childCy - containerCy),
  732 |         dimmedCount,
  733 |       };
  734 |     }, candidate.childId);
  735 | 
  736 |     expect(focusedMetrics, 'focused graph metrics must be measurable').not.toBeNull();
  737 |     expect(
  738 |       focusedMetrics!.childDistance,
  739 |       'focused node should move closer to the graph center after selection',
  740 |     ).toBeLessThanOrEqual(Math.max(48, initialDistance! * 0.75));
  741 |     expect(
  742 |       focusedMetrics!.dimmedCount,
  743 |       'graph selection should dim at least one unrelated visible node',
  744 |     ).toBeGreaterThan(0);
  745 | 
  746 |     await attachScreenshot(page, testInfo, 'graph-focused-selection');
  747 |   });
  748 | 
  749 |   test('graph node LOD keeps the active selection rich while collapsing other visible nodes', async ({ page }, testInfo) => {
  750 |     test.setTimeout(120_000);
  751 | 
  752 |     const candidate = await findGraphSelectionCandidate(page);
  753 | 
  754 |     await openCandidateTicket(page, candidate);
  755 | 
  756 |     const container = page.locator('#graph3d-container');
  757 |     await page.getByRole('button', { name: /^Graph$/ }).first().click();
  758 |     await expect(container).toBeVisible({ timeout: 30_000 });
  759 | 
  760 |     await expect.poll(() => graphLodMetrics(page, candidate.rootId), {
  761 |       timeout: 20_000,
  762 |     }).toMatchObject({
  763 |       selectedLod: 'rich',
  764 |       visibleNodeCount: expect.any(Number),
  765 |     });
  766 | 
  767 |     const initialMetrics = await graphLodMetrics(page, candidate.rootId);
  768 |     console.info('LOD baseline', await page.evaluate(() => {
  769 |       const container = document.getElementById('graph3d-container');
  770 |       return {
  771 |         cameraDistance: container?.getAttribute('data-camera-distance'),
  772 |         cameraTarget: container?.getAttribute('data-camera-target'),
  773 |         nodes: Array.from(container?.querySelectorAll('[data-node-id]') ?? []).map((node) => {
  774 |           const element = node as HTMLElement;
  775 |           const rect = element.getBoundingClientRect();
  776 |           return {
  777 |             id: element.dataset.nodeId,
  778 |             lod: element.getAttribute('data-node-lod'),
  779 |             display: element.style.display,
  780 |             width: rect.width,
  781 |             height: rect.height,
  782 |             transform: element.style.transform,
  783 |           };
  784 |         }),
  785 |       };
  786 |     }));
  787 |     expect(initialMetrics, 'root graph node should expose LOD metrics').not.toBeNull();
  788 |     expect(
  789 |       initialMetrics!.collapsedNodes,
  790 |       'workspace graph should collapse at least one non-selected visible node to a smaller LOD tier',
> 791 |     ).toBeGreaterThan(0);
      |       ^ Error: workspace graph should collapse at least one non-selected visible node to a smaller LOD tier
  792 | 
  793 |     await zoomGraph(page, 480, 6);
  794 | 
  795 |     await expect.poll(() => graphLodMetrics(page, candidate.rootId), {
  796 |       timeout: 20_000,
  797 |     }).toMatchObject({
  798 |       selectedLod: 'rich',
  799 |       visibleNodeCount: expect.any(Number),
  800 |     });
  801 | 
  802 |     const zoomedOutMetrics = await graphLodMetrics(page, candidate.rootId);
  803 |     expect(zoomedOutMetrics, 'zoomed-out root graph node should expose LOD metrics').not.toBeNull();
  804 |     expect(
  805 |       zoomedOutMetrics!.collapsedNodes,
  806 |       'zooming out should keep at least one non-selected visible node in a smaller LOD tier',
  807 |     ).toBeGreaterThan(0);
  808 | 
  809 |     const childNode = page.locator(`#graph3d-container [data-node-id="${candidate.childId}"]`).first();
  810 |     await expect(childNode).toBeVisible();
  811 | 
  812 |     const zoomedOutChildLod = await graphNodeLod(page, candidate.childId);
  813 |     expect(
  814 |       zoomedOutChildLod === 'compact' || zoomedOutChildLod === 'minimal',
  815 |       'zoomed-out child node should remain interactive while rendered in a smaller LOD tier',
  816 |     ).toBe(true);
  817 | 
  818 |     await childNode.click();
  819 | 
  820 |     await expect.poll(() => graphLodMetrics(page, candidate.childId), {
  821 |       timeout: 20_000,
  822 |     }).toMatchObject({
  823 |       selectedLod: 'rich',
  824 |       visibleNodeCount: expect.any(Number),
  825 |     });
  826 | 
  827 |     const childMetrics = await graphLodMetrics(page, candidate.childId);
  828 |     expect(childMetrics, 'selected child node should expose LOD metrics').not.toBeNull();
  829 |     expect(
  830 |       childMetrics!.collapsedNodes,
  831 |       'after selection, other visible nodes should still use smaller compact or minimal tiers',
  832 |     ).toBeGreaterThan(0);
  833 | 
  834 |     await zoomGraph(page, -480, 4);
  835 | 
  836 |     await expect.poll(() => graphLodMetrics(page, candidate.childId), {
  837 |       timeout: 20_000,
  838 |     }).toMatchObject({
  839 |       selectedLod: 'rich',
  840 |       visibleNodeCount: expect.any(Number),
  841 |     });
  842 | 
  843 |     const zoomedInMetrics = await graphLodMetrics(page, candidate.childId);
  844 |     expect(zoomedInMetrics, 'zoomed-in child graph node should expose LOD metrics').not.toBeNull();
  845 |     expect(
  846 |       zoomedInMetrics!.minimalNodes,
  847 |       'zooming back in should not increase the number of minimal visible nodes',
  848 |     ).toBeLessThanOrEqual(zoomedOutMetrics!.minimalNodes);
  849 | 
  850 |     await attachScreenshot(page, testInfo, 'graph-node-lod-tiers');
  851 |   });
  852 | 
  853 |   test('dragged graph layout and camera zoom persist when focus changes inside the same graph', async ({ page }, testInfo) => {
  854 |     test.setTimeout(120_000);
  855 | 
  856 |     const candidate = await findGraphSelectionCandidate(page);
  857 | 
  858 |     await openCandidateTicket(page, candidate);
  859 | 
  860 |     await page.getByRole('button', { name: /^Graph$/ }).first().click();
  861 |     await expect(page.locator('#graph3d-container')).toBeVisible({ timeout: 30_000 });
  862 | 
  863 |     await expect.poll(() => graphNodeMetrics(page, candidate.rootId), {
  864 |       timeout: 30_000,
  865 |     }).not.toBeNull();
  866 |     await expect.poll(() => graphNodeMetrics(page, candidate.childId), {
  867 |       timeout: 30_000,
  868 |     }).not.toBeNull();
  869 | 
  870 |     const childBeforeDrag = await graphNodeMetrics(page, candidate.childId);
  871 |     expect(childBeforeDrag, 'child graph node should expose drag metrics before movement').not.toBeNull();
  872 | 
  873 |     await dragGraphNode(page, candidate.childId, 120, 60);
  874 | 
  875 |     await expect.poll(async () => {
  876 |       const metrics = await graphNodeMetrics(page, candidate.childId);
  877 |       if (!metrics || !childBeforeDrag) {
  878 |         return 0;
  879 |       }
  880 |       return Math.hypot(
  881 |         metrics.centerX - childBeforeDrag.centerX,
  882 |         metrics.centerY - childBeforeDrag.centerY,
  883 |       );
  884 |     }, {
  885 |       timeout: 20_000,
  886 |     }).toBeGreaterThan(40);
  887 | 
  888 |     await zoomGraph(page, 480, 4);
  889 | 
  890 |     const rootBeforeFocus = await graphNodeMetrics(page, candidate.rootId);
  891 |     const childBeforeFocus = await graphNodeMetrics(page, candidate.childId);
```
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
        - button "Graph" [active] [ref=e62] [cursor=pointer]
        - button "Split" [ref=e63] [cursor=pointer]
        - button "Content" [ref=e64] [cursor=pointer]
      - generic [ref=e67]:
        - img
        - generic:
          - generic:
            - generic [ref=e69] [cursor=pointer]:
              - generic [ref=e71]: planning
              - generic [ref=e72]: Release E2E search fixture alpha
              - generic [ref=e73]:
                - generic [ref=e74]: Ticket
                - generic [ref=e75]: f32e9df7
            - generic [ref=e77] [cursor=pointer]:
              - generic [ref=e79]: planning
              - generic [ref=e80]: Release E2E search fixture gamma
              - generic [ref=e81]:
                - generic [ref=e82]: Ticket
                - generic [ref=e83]: 0b4337f6
            - generic [ref=e85] [cursor=pointer]:
              - generic [ref=e87]: planning
              - generic [ref=e88]: Release E2E legacy description fixture
              - generic [ref=e89]:
                - generic [ref=e90]: Ticket
                - generic [ref=e91]: 3a1ec9f8
            - generic [ref=e93] [cursor=pointer]:
              - generic [ref=e95]: planning
              - generic [ref=e96]: Release E2E search fixture epsilon
              - generic [ref=e97]:
                - generic [ref=e98]: Ticket
                - generic [ref=e99]: 0e7da84e
            - generic [ref=e101] [cursor=pointer]:
              - generic [ref=e103]: planning
              - generic [ref=e104]: Release E2E search fixture beta
              - generic [ref=e105]:
                - generic [ref=e106]: Ticket
                - generic [ref=e107]: cbf4d945
            - generic [ref=e109] [cursor=pointer]:
              - generic [ref=e111]: planning
              - generic [ref=e112]: Release E2E search fixture delta
              - generic [ref=e113]:
                - generic [ref=e114]: Ticket
                - generic [ref=e115]: 5ea85741
            - generic [ref=e117] [cursor=pointer]:
              - generic [ref=e119]: planning
              - generic [ref=e120]: Release E2E navigation fixture
              - generic [ref=e121]:
                - generic [ref=e122]: Ticket
                - generic [ref=e123]: e7a9252f
            - generic [ref=e125] [cursor=pointer]:
              - generic [ref=e127]: planning
              - generic [ref=e128]: Release E2E graph root
              - generic [ref=e129]:
                - generic [ref=e130]: Ticket
                - generic [ref=e131]: b3a60766
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
              - generic [ref=e151]: in-review
              - generic [ref=e152]: Release E2E review prerequisite
              - generic [ref=e153]:
                - generic [ref=e154]: Ticket
                - generic [ref=e155]: 68e2a319
            - generic [ref=e157] [cursor=pointer]:
              - generic [ref=e159]: in-implementation
              - generic [ref=e160]: Release E2E implementation prerequisite
              - generic [ref=e161]:
                - generic [ref=e162]: Ticket
                - generic [ref=e163]: 4dfa31b5
          - generic: "Left-drag: orbit · Right-drag: pan · Scroll: zoom · Click card: open"
          - generic: 12 nodes
        - button "⚙" [ref=e165] [cursor=pointer]
```

# Test source

```ts
  672 |       const visibleNodes = Array.from(container.querySelectorAll('[data-node-id]')).filter((node) => {
  673 |         const element = node as HTMLElement;
  674 |         return element.style.display !== 'none';
  675 |       }) as HTMLElement[];
  676 |       const child = visibleNodes.find((node) => node.dataset.nodeId === childId);
  677 |       if (!child) {
  678 |         return false;
  679 |       }
  680 | 
  681 |       const containerRect = container.getBoundingClientRect();
  682 |       const childRect = child.getBoundingClientRect();
  683 |       const containerCx = containerRect.left + containerRect.width / 2;
  684 |       const containerCy = containerRect.top + containerRect.height / 2;
  685 |       const childCx = childRect.left + childRect.width / 2;
  686 |       const childCy = childRect.top + childRect.height / 2;
  687 |       const dimmedCount = visibleNodes.filter((node) => {
  688 |         if (node.dataset.nodeId === childId) {
  689 |           return false;
  690 |         }
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
  768 |     expect(initialMetrics, 'root graph node should expose LOD metrics').not.toBeNull();
  769 |     expect(
  770 |       initialMetrics!.collapsedNodes,
  771 |       'workspace graph should collapse at least one non-selected visible node to a smaller LOD tier',
> 772 |     ).toBeGreaterThan(0);
      |       ^ Error: workspace graph should collapse at least one non-selected visible node to a smaller LOD tier
  773 | 
  774 |     await zoomGraph(page, 480, 6);
  775 | 
  776 |     await expect.poll(() => graphLodMetrics(page, candidate.rootId), {
  777 |       timeout: 20_000,
  778 |     }).toMatchObject({
  779 |       selectedLod: 'rich',
  780 |       visibleNodeCount: expect.any(Number),
  781 |     });
  782 | 
  783 |     const zoomedOutMetrics = await graphLodMetrics(page, candidate.rootId);
  784 |     expect(zoomedOutMetrics, 'zoomed-out root graph node should expose LOD metrics').not.toBeNull();
  785 |     expect(
  786 |       zoomedOutMetrics!.collapsedNodes,
  787 |       'zooming out should keep at least one non-selected visible node in a smaller LOD tier',
  788 |     ).toBeGreaterThan(0);
  789 | 
  790 |     const childNode = page.locator(`#graph3d-container [data-node-id="${candidate.childId}"]`).first();
  791 |     await expect(childNode).toBeVisible();
  792 | 
  793 |     const zoomedOutChildLod = await graphNodeLod(page, candidate.childId);
  794 |     expect(
  795 |       zoomedOutChildLod === 'compact' || zoomedOutChildLod === 'minimal',
  796 |       'zoomed-out child node should remain interactive while rendered in a smaller LOD tier',
  797 |     ).toBe(true);
  798 | 
  799 |     await childNode.click();
  800 | 
  801 |     await expect.poll(() => graphLodMetrics(page, candidate.childId), {
  802 |       timeout: 20_000,
  803 |     }).toMatchObject({
  804 |       selectedLod: 'rich',
  805 |       visibleNodeCount: expect.any(Number),
  806 |     });
  807 | 
  808 |     const childMetrics = await graphLodMetrics(page, candidate.childId);
  809 |     expect(childMetrics, 'selected child node should expose LOD metrics').not.toBeNull();
  810 |     expect(
  811 |       childMetrics!.collapsedNodes,
  812 |       'after selection, other visible nodes should still use smaller compact or minimal tiers',
  813 |     ).toBeGreaterThan(0);
  814 | 
  815 |     await zoomGraph(page, -480, 4);
  816 | 
  817 |     await expect.poll(() => graphLodMetrics(page, candidate.childId), {
  818 |       timeout: 20_000,
  819 |     }).toMatchObject({
  820 |       selectedLod: 'rich',
  821 |       visibleNodeCount: expect.any(Number),
  822 |     });
  823 | 
  824 |     const zoomedInMetrics = await graphLodMetrics(page, candidate.childId);
  825 |     expect(zoomedInMetrics, 'zoomed-in child graph node should expose LOD metrics').not.toBeNull();
  826 |     expect(
  827 |       zoomedInMetrics!.minimalNodes,
  828 |       'zooming back in should not increase the number of minimal visible nodes',
  829 |     ).toBeLessThanOrEqual(zoomedOutMetrics!.minimalNodes);
  830 | 
  831 |     await attachScreenshot(page, testInfo, 'graph-node-lod-tiers');
  832 |   });
  833 | 
  834 |   test('dragged graph layout and camera zoom persist when focus changes inside the same graph', async ({ page }, testInfo) => {
  835 |     test.setTimeout(120_000);
  836 | 
  837 |     const candidate = await findGraphSelectionCandidate(page);
  838 | 
  839 |     await openCandidateTicket(page, candidate);
  840 | 
  841 |     await page.getByRole('button', { name: /^Graph$/ }).first().click();
  842 |     await expect(page.locator('#graph3d-container')).toBeVisible({ timeout: 30_000 });
  843 | 
  844 |     await expect.poll(() => graphNodeMetrics(page, candidate.rootId), {
  845 |       timeout: 30_000,
  846 |     }).not.toBeNull();
  847 |     await expect.poll(() => graphNodeMetrics(page, candidate.childId), {
  848 |       timeout: 30_000,
  849 |     }).not.toBeNull();
  850 | 
  851 |     const childBeforeDrag = await graphNodeMetrics(page, candidate.childId);
  852 |     expect(childBeforeDrag, 'child graph node should expose drag metrics before movement').not.toBeNull();
  853 | 
  854 |     await dragGraphNode(page, candidate.childId, 120, 60);
  855 | 
  856 |     await expect.poll(async () => {
  857 |       const metrics = await graphNodeMetrics(page, candidate.childId);
  858 |       if (!metrics || !childBeforeDrag) {
  859 |         return 0;
  860 |       }
  861 |       return Math.hypot(
  862 |         metrics.centerX - childBeforeDrag.centerX,
  863 |         metrics.centerY - childBeforeDrag.centerY,
  864 |       );
  865 |     }, {
  866 |       timeout: 20_000,
  867 |     }).toBeGreaterThan(40);
  868 | 
  869 |     await zoomGraph(page, 480, 4);
  870 | 
  871 |     const rootBeforeFocus = await graphNodeMetrics(page, candidate.rootId);
  872 |     const childBeforeFocus = await graphNodeMetrics(page, candidate.childId);
```
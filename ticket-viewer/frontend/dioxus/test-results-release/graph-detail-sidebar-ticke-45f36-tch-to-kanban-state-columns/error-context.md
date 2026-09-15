# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: graph-detail-sidebar.spec.ts >> ticket-viewer — graph selection updates right detail sidebar >> graph settings can switch to kanban state columns
- Location: e2e-release\graph-detail-sidebar.spec.ts:1043:7

# Error details

```
Error: expect(received).toBeLessThan(expected)

Expected: < 2.5
Received:   8.48992919921875

Call Log:
- Timeout 30000ms exceeded while waiting on the predicate
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
            - textbox "Search titles/descriptions or use id:, title:, state:, type:" [ref=e36]: id:e108982f-f90f-40d4-be1d-fdc30475f3dc
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
            - button "Release E2E completed prerequisite done" [ref=e54] [cursor=pointer]:
              - generic [ref=e56]: Release E2E completed prerequisite
              - generic [ref=e57]: done
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
            - generic: In Implementation
            - generic: In Review
            - generic: Done
            - generic: Planning
            - generic: Ready
            - generic: 01. Release E2E graph root
            - generic: 02. Release E2E legacy description fixture
            - generic: 03. Release E2E navigation fixture
            - generic: 04. Release E2E search fixture alpha
            - generic: 05. Release E2E search fixture beta
            - generic: 06. Release E2E search fixture delta
            - generic: 07. Release E2E search fixture epsilon
            - generic: 08. Release E2E search fixture gamma
            - generic: 09. Release E2E implementation prerequisite
            - generic: 10. Release E2E review prerequisite
            - generic: 11. Release E2E completed prerequisite
            - generic: 12. Release E2E ready prerequisite
          - generic:
            - generic [ref=e70] [cursor=pointer]: P
            - generic [ref=e73] [cursor=pointer]: P
            - generic [ref=e76] [cursor=pointer]: P
            - generic [ref=e79] [cursor=pointer]: P
            - generic [ref=e82] [cursor=pointer]: P
            - generic [ref=e85] [cursor=pointer]: P
            - generic [ref=e88] [cursor=pointer]: P
            - generic [ref=e91] [cursor=pointer]: P
            - generic [ref=e94] [cursor=pointer]: I
            - generic [ref=e97] [cursor=pointer]: R
            - generic [ref=e101] [cursor=pointer]: 4dfa31b5
            - generic [ref=e103] [cursor=pointer]:
              - generic [ref=e105]: done
              - generic [ref=e106]: Release E2E completed prerequisite
              - generic [ref=e107]:
                - generic [ref=e108]: Ticket
                - generic [ref=e109]: e108982f
          - generic: "Left-drag: orbit · Right-drag: pan · Scroll: zoom · Click card: open"
          - generic: 12 nodes
        - generic [ref=e110]:
          - generic [ref=e111]:
            - generic [ref=e112]: Layout
            - generic [ref=e113]:
              - button "Hierarchical 3D" [ref=e114] [cursor=pointer]
              - button "Flat 2D" [ref=e115] [cursor=pointer]
              - button "Kanban" [active] [ref=e116] [cursor=pointer]
              - button "Fixed 2D" [ref=e117] [cursor=pointer]
            - generic [ref=e118]: Projection
            - generic [ref=e119]:
              - button "Perspective" [ref=e120] [cursor=pointer]
              - button "Orthographic" [ref=e121] [cursor=pointer]
            - generic [ref=e122]: Edge theme
            - generic [ref=e123]:
              - generic [ref=e124]:
                - generic [ref=e125]: Overlay opacity
                - generic [ref=e126]: 80%
              - slider [ref=e127]: "0.8"
            - generic [ref=e128]: Blend mode
            - generic [ref=e129]:
              - button "Screen" [ref=e130] [cursor=pointer]
              - button "Plus lighter" [ref=e131] [cursor=pointer]
              - button "Normal" [ref=e132] [cursor=pointer]
            - generic [ref=e133] [cursor=pointer]:
              - generic [ref=e134]: Dependency edge
              - textbox "Dependency edge" [ref=e136]: "#47dbff"
            - generic [ref=e137] [cursor=pointer]:
              - generic [ref=e138]: Blocking edge
              - textbox "Blocking edge" [ref=e140]: "#ff8f47"
            - generic [ref=e141] [cursor=pointer]:
              - generic [ref=e142]: Structural edge
              - textbox "Structural edge" [ref=e144]: "#c29eff"
            - generic [ref=e145] [cursor=pointer]:
              - generic [ref=e146]: Default edge
              - textbox "Default edge" [ref=e148]: "#c7d1ff"
            - generic [ref=e149]: Node theme
            - generic [ref=e150] [cursor=pointer]:
              - generic [ref=e151]: Card surface
              - textbox "Card surface" [ref=e153]: "#1c212e"
            - generic [ref=e154] [cursor=pointer]:
              - generic [ref=e155]: Card border
              - textbox "Card border" [ref=e157]: "#ccd9f2"
            - generic [ref=e158] [cursor=pointer]:
              - generic [ref=e159]: Card text
              - textbox "Card text" [ref=e161]: "#f5f7ff"
            - generic [ref=e162]:
              - generic [ref=e163]:
                - generic [ref=e164]: Shadow strength
                - generic [ref=e165]: 32%
              - slider [ref=e166]: "0.32"
          - button "⚙" [ref=e167] [cursor=pointer]
```

# Test source

```ts
  997  | 
  998  |     await page.getByTitle('Graph settings').click();
  999  | 
  1000 |     const hierarchicalButton = page.getByRole('button', { name: 'Hierarchical 3D' });
  1001 |     const flatButton = page.getByRole('button', { name: 'Flat 2D' });
  1002 |     const perspectiveButton = page.getByRole('button', { name: 'Perspective' });
  1003 |     const orthographicButton = page.getByRole('button', { name: 'Orthographic' });
  1004 | 
  1005 |     await expect(hierarchicalButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);
  1006 |     await expect(orthographicButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);
  1007 | 
  1008 |     await flatButton.click();
  1009 |     await perspectiveButton.click();
  1010 | 
  1011 |     await expect(flatButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);
  1012 |     await expect(perspectiveButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);
  1013 | 
  1014 |     const switchedMetrics = await graphHierarchyMetrics(page, candidate.rootId, candidate.childId);
  1015 |     expect(switchedMetrics, 'graph should stay mounted after switching layout and projection').not.toBeNull();
  1016 |     expect(
  1017 |       switchedMetrics!.visibleNodeCount,
  1018 |       'root and child nodes should remain visible after switching away from defaults',
  1019 |     ).toBeGreaterThanOrEqual(2);
  1020 | 
  1021 |     await hierarchicalButton.click();
  1022 |     await orthographicButton.click();
  1023 | 
  1024 |     await expect(hierarchicalButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);
  1025 |     await expect(orthographicButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);
  1026 | 
  1027 |     await expect.poll(() => graphHierarchyMetrics(page, candidate.rootId, candidate.childId), {
  1028 |       timeout: 20_000,
  1029 |     }).toMatchObject({
  1030 |       visibleNodeCount: expect.any(Number),
  1031 |     });
  1032 | 
  1033 |     const restoredMetrics = await graphHierarchyMetrics(page, candidate.rootId, candidate.childId);
  1034 |     expect(restoredMetrics, 'restored default graph layout should expose root and child positions').not.toBeNull();
  1035 |     expect(
  1036 |       restoredMetrics!.childY,
  1037 |       'restoring the default layout should preserve top-to-bottom hierarchy ordering',
  1038 |     ).toBeGreaterThan(restoredMetrics!.rootY + 10);
  1039 | 
  1040 |     await attachScreenshot(page, testInfo, 'graph-restored-default-layout');
  1041 |   });
  1042 | 
  1043 |   test('graph settings can switch to kanban state columns', async ({ page }, testInfo) => {
  1044 |     test.setTimeout(180_000);
  1045 | 
  1046 |     const candidate = await findKanbanLayoutCandidate(page);
  1047 | 
  1048 |     await openTicketById(page, candidate.workspace, candidate.rootId);
  1049 | 
  1050 |     await page.getByRole('button', { name: /^Graph$/ }).first().click();
  1051 |     await expect(page.locator('#graph3d-container')).toBeVisible({ timeout: 30_000 });
  1052 | 
  1053 |     await page.getByTitle('Graph settings').click();
  1054 |     const kanbanButton = page.getByRole('button', { name: 'Kanban' });
  1055 |     await expect(kanbanButton).toBeVisible();
  1056 |     await kanbanButton.click();
  1057 |     await expect(kanbanButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);
  1058 |     await attachScreenshot(page, testInfo, 'graph-kanban-guides-after-switch');
  1059 | 
  1060 |     for (const sample of candidate.stateSamples) {
  1061 |       await expect.poll(
  1062 |         () => visibleGuideCount(page, `#graph3d-container [data-kanban-column-header="${sample.state}"]`),
  1063 |         { timeout: 30_000 },
  1064 |       ).toBeGreaterThan(0);
  1065 |     }
  1066 | 
  1067 |     await expect.poll(
  1068 |       () => visibleGuideCount(page, '#graph3d-container [data-kanban-column-separator]'),
  1069 |       { timeout: 30_000 },
  1070 |     ).toBeGreaterThan(0);
  1071 |     await expect.poll(
  1072 |       () => visibleGuideCount(page, '#graph3d-container [data-kanban-row-label]'),
  1073 |       { timeout: 30_000 },
  1074 |     ).toBeGreaterThan(0);
  1075 | 
  1076 |     await expect.poll(
  1077 |       async () => (await visibleGuideMetrics(page, '#graph3d-container [data-kanban-row-label]'))?.height ?? 0,
  1078 |       { timeout: 30_000 },
  1079 |     ).toBeGreaterThan(0);
  1080 |     await expect.poll(() => visibleRowLabelOverlapsVisibleNode(page), {
  1081 |       timeout: 30_000,
  1082 |     }).toBe(false);
  1083 | 
  1084 |     await zoomGraph(page, -480, 3);
  1085 | 
  1086 |     await expect.poll(
  1087 |       async () => (await visibleGuideMetrics(page, '#graph3d-container [data-kanban-row-label]'))?.height ?? 0,
  1088 |       { timeout: 30_000 },
  1089 |     ).toBeGreaterThan(5.0);
  1090 | 
  1091 |     await expect.poll(() => visibleRowLabelOverlapsVisibleNode(page), {
  1092 |       timeout: 30_000,
  1093 |     }).toBe(false);
  1094 | 
  1095 |     await zoomGraph(page, 480, 7);
  1096 | 
> 1097 |     await expect.poll(
       |     ^ Error: expect(received).toBeLessThan(expected)
  1098 |       async () => (await visibleGuideMetrics(page, '#graph3d-container [data-kanban-row-label]'))?.height ?? Number.POSITIVE_INFINITY,
  1099 |       { timeout: 30_000 },
  1100 |     ).toBeLessThan(2.5);
  1101 | 
  1102 |     await expect.poll(async () => {
  1103 |       const metrics = await Promise.all(
  1104 |         candidate.stateSamples.map((sample) => graphNodeMetrics(page, sample.id)),
  1105 |       );
  1106 |       return metrics.every((sample) => sample !== null);
  1107 |     }, {
  1108 |       timeout: 30_000,
  1109 |     }).toBe(true);
  1110 | 
  1111 |     const orderedSamples = (await Promise.all(
  1112 |       candidate.stateSamples.map(async (sample) => ({
  1113 |         ...sample,
  1114 |         metrics: await graphNodeMetrics(page, sample.id),
  1115 |       })),
  1116 |     ))
  1117 |       .filter((sample): sample is typeof sample & { metrics: GraphNodeMetrics } => sample.metrics !== null)
  1118 |       .sort((left, right) => {
  1119 |         const rankDelta = kanbanStateRank(left.state) - kanbanStateRank(right.state);
  1120 |         return rankDelta === 0 ? left.state.localeCompare(right.state) : rankDelta;
  1121 |       });
  1122 | 
  1123 |     expect(
  1124 |       orderedSamples.length,
  1125 |       'kanban candidate should provide visible state samples after switching layouts',
  1126 |     ).toBeGreaterThanOrEqual(2);
  1127 | 
  1128 |     for (let index = 1; index < orderedSamples.length; index += 1) {
  1129 |       expect(
  1130 |         orderedSamples[index].metrics.layoutX,
  1131 |         `state ${orderedSamples[index].state} should render to the right of ${orderedSamples[index - 1].state} in kanban mode`,
  1132 |       ).not.toBeNull();
  1133 |       expect(
  1134 |         orderedSamples[index - 1].metrics.layoutX,
  1135 |         `state ${orderedSamples[index - 1].state} should expose a layout x-coordinate in kanban mode`,
  1136 |       ).not.toBeNull();
  1137 |       expect(
  1138 |         orderedSamples[index].metrics.layoutX!,
  1139 |         `state ${orderedSamples[index].state} should render to the right of ${orderedSamples[index - 1].state} in kanban mode`,
  1140 |       ).toBeGreaterThan(orderedSamples[index - 1].metrics.layoutX! + 0.25);
  1141 |     }
  1142 | 
  1143 |     await attachScreenshot(page, testInfo, 'graph-kanban-state-columns');
  1144 |   });
  1145 | });
```
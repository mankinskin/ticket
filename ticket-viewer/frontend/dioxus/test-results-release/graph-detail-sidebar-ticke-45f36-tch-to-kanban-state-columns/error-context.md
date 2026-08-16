# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: graph-detail-sidebar.spec.ts >> ticket-viewer — graph selection updates right detail sidebar >> graph settings can switch to kanban state columns
- Location: e2e-release\graph-detail-sidebar.spec.ts:1062:7

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
            - textbox "Search titles/descriptions or use id:, title:, state:, type:" [ref=e36]: id:e5d14cdc-8da5-4ca1-aa73-f85c5ef4c105
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
            - button "Release E2E review prerequisite in-review" [ref=e53] [cursor=pointer]:
              - generic [ref=e55]: Release E2E review prerequisite
              - generic [ref=e56]: in-review
      - separator "Resize panel" [ref=e57]
    - generic [ref=e58]:
      - generic [ref=e59]:
        - generic [ref=e60]: "View:"
        - button "Graph" [ref=e61] [cursor=pointer]
        - button "Split" [ref=e62] [cursor=pointer]
        - button "Content" [ref=e63] [cursor=pointer]
      - generic [ref=e66]:
        - img
        - generic:
          - generic:
            - generic: Open
            - generic: Planned
            - generic: In Implementation
            - generic: In Review
            - generic: Done
            - generic: 01. Release E2E graph root
            - generic: 02. Release E2E legacy description fixture
            - generic: 03. Release E2E navigation fixture
            - generic: 04. Release E2E search fixture alpha
            - generic: 05. Release E2E search fixture beta
            - generic: 06. Release E2E search fixture delta
            - generic: 07. Release E2E search fixture epsilon
            - generic: 08. Release E2E search fixture gamma
            - generic: 09. Release E2E ready prerequisite
            - generic: 10. Release E2E implementation prerequisite
            - generic: 11. Release E2E review prerequisite
            - generic: 12. Release E2E completed prerequisite
          - generic:
            - generic [ref=e69] [cursor=pointer]: O
            - generic [ref=e72] [cursor=pointer]: O
            - generic [ref=e75] [cursor=pointer]: O
            - generic [ref=e78] [cursor=pointer]: O
            - generic [ref=e81] [cursor=pointer]: O
            - generic [ref=e84] [cursor=pointer]: O
            - generic [ref=e87] [cursor=pointer]: O
            - generic [ref=e90] [cursor=pointer]: O
            - generic [ref=e93] [cursor=pointer]: P
            - generic [ref=e96] [cursor=pointer]: D
            - generic [ref=e99] [cursor=pointer]: I
            - generic [ref=e101] [cursor=pointer]:
              - generic [ref=e103]: in-review
              - generic [ref=e104]: Release E2E review prerequisite
              - generic [ref=e105]:
                - generic [ref=e106]: Ticket
                - generic [ref=e107]: e5d14cdc
          - generic: "Left-drag: orbit · Right-drag: pan · Scroll: zoom · Click card: open"
          - generic: 12 nodes
        - generic [ref=e108]:
          - generic [ref=e109]:
            - generic [ref=e110]: Layout
            - generic [ref=e111]:
              - button "Hierarchical 3D" [ref=e112] [cursor=pointer]
              - button "Flat 2D" [ref=e113] [cursor=pointer]
              - button "Kanban" [active] [ref=e114] [cursor=pointer]
              - button "Fixed 2D" [ref=e115] [cursor=pointer]
            - generic [ref=e116]: Projection
            - generic [ref=e117]:
              - button "Perspective" [ref=e118] [cursor=pointer]
              - button "Orthographic" [ref=e119] [cursor=pointer]
            - generic [ref=e120]: Edge theme
            - generic [ref=e121]:
              - generic [ref=e122]:
                - generic [ref=e123]: Overlay opacity
                - generic [ref=e124]: 80%
              - slider [ref=e125]: "0.8"
            - generic [ref=e126]: Blend mode
            - generic [ref=e127]:
              - button "Screen" [ref=e128] [cursor=pointer]
              - button "Plus lighter" [ref=e129] [cursor=pointer]
              - button "Normal" [ref=e130] [cursor=pointer]
            - generic [ref=e131] [cursor=pointer]:
              - generic [ref=e132]: Dependency edge
              - textbox "Dependency edge" [ref=e134]: "#47dbff"
            - generic [ref=e135] [cursor=pointer]:
              - generic [ref=e136]: Blocking edge
              - textbox "Blocking edge" [ref=e138]: "#ff8f47"
            - generic [ref=e139] [cursor=pointer]:
              - generic [ref=e140]: Structural edge
              - textbox "Structural edge" [ref=e142]: "#c29eff"
            - generic [ref=e143] [cursor=pointer]:
              - generic [ref=e144]: Default edge
              - textbox "Default edge" [ref=e146]: "#c7d1ff"
            - generic [ref=e147]: Node theme
            - generic [ref=e148] [cursor=pointer]:
              - generic [ref=e149]: Card surface
              - textbox "Card surface" [ref=e151]: "#1c212e"
            - generic [ref=e152] [cursor=pointer]:
              - generic [ref=e153]: Card border
              - textbox "Card border" [ref=e155]: "#ccd9f2"
            - generic [ref=e156] [cursor=pointer]:
              - generic [ref=e157]: Card text
              - textbox "Card text" [ref=e159]: "#f5f7ff"
            - generic [ref=e160]:
              - generic [ref=e161]:
                - generic [ref=e162]: Shadow strength
                - generic [ref=e163]: 32%
              - slider [ref=e164]: "0.32"
          - button "⚙" [ref=e165] [cursor=pointer]
```

# Test source

```ts
  1034 |     expect(switchedMetrics, 'graph should stay mounted after switching layout and projection').not.toBeNull();
  1035 |     expect(
  1036 |       switchedMetrics!.visibleNodeCount,
  1037 |       'root and child nodes should remain visible after switching away from defaults',
  1038 |     ).toBeGreaterThanOrEqual(2);
  1039 | 
  1040 |     await hierarchicalButton.click();
  1041 |     await orthographicButton.click();
  1042 | 
  1043 |     await expect(hierarchicalButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);
  1044 |     await expect(orthographicButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);
  1045 | 
  1046 |     await expect.poll(() => graphHierarchyMetrics(page, candidate.rootId, candidate.childId), {
  1047 |       timeout: 20_000,
  1048 |     }).toMatchObject({
  1049 |       visibleNodeCount: expect.any(Number),
  1050 |     });
  1051 | 
  1052 |     const restoredMetrics = await graphHierarchyMetrics(page, candidate.rootId, candidate.childId);
  1053 |     expect(restoredMetrics, 'restored default graph layout should expose root and child positions').not.toBeNull();
  1054 |     expect(
  1055 |       restoredMetrics!.childY,
  1056 |       'restoring the default layout should preserve top-to-bottom hierarchy ordering',
  1057 |     ).toBeGreaterThan(restoredMetrics!.rootY + 10);
  1058 | 
  1059 |     await attachScreenshot(page, testInfo, 'graph-restored-default-layout');
  1060 |   });
  1061 | 
  1062 |   test('graph settings can switch to kanban state columns', async ({ page }, testInfo) => {
  1063 |     test.setTimeout(180_000);
  1064 | 
  1065 |     const candidate = await findKanbanLayoutCandidate(page);
  1066 | 
  1067 |     await openTicketById(page, candidate.workspace, candidate.rootId);
  1068 | 
  1069 |     await page.getByRole('button', { name: /^Graph$/ }).first().click();
  1070 |     await expect(page.locator('#graph3d-container')).toBeVisible({ timeout: 30_000 });
  1071 | 
  1072 |     await page.getByTitle('Graph settings').click();
  1073 |     const kanbanButton = page.getByRole('button', { name: 'Kanban' });
  1074 |     await expect(kanbanButton).toBeVisible();
  1075 |     await kanbanButton.click();
  1076 |     await expect(kanbanButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);
  1077 |     await attachScreenshot(page, testInfo, 'graph-kanban-guides-after-switch');
  1078 | 
  1079 |     for (const sample of candidate.stateSamples) {
  1080 |       await expect.poll(
  1081 |         () => visibleGuideCount(page, `#graph3d-container [data-kanban-column-header="${sample.state}"]`),
  1082 |         { timeout: 30_000 },
  1083 |       ).toBeGreaterThan(0);
  1084 |     }
  1085 | 
  1086 |     await expect.poll(
  1087 |       () => visibleGuideCount(page, '#graph3d-container [data-kanban-column-separator]'),
  1088 |       { timeout: 30_000 },
  1089 |     ).toBeGreaterThan(0);
  1090 |     await expect.poll(
  1091 |       () => visibleGuideCount(page, '#graph3d-container [data-kanban-row-label]'),
  1092 |       { timeout: 30_000 },
  1093 |     ).toBeGreaterThan(0);
  1094 | 
  1095 |     await expect.poll(
  1096 |       async () => (await visibleGuideMetrics(page, '#graph3d-container [data-kanban-row-label]'))?.height ?? 0,
  1097 |       { timeout: 30_000 },
  1098 |     ).toBeGreaterThan(0);
  1099 |     await expect.poll(() => visibleRowLabelOverlapsVisibleNode(page), {
  1100 |       timeout: 30_000,
  1101 |     }).toBe(false);
  1102 | 
  1103 |     await zoomGraph(page, -480, 3);
  1104 | 
  1105 |     await expect.poll(
  1106 |       async () => (await visibleGuideMetrics(page, '#graph3d-container [data-kanban-row-label]'))?.height ?? 0,
  1107 |       { timeout: 30_000 },
  1108 |     ).toBeGreaterThan(5.0);
  1109 | 
  1110 |     await expect.poll(() => visibleRowLabelOverlapsVisibleNode(page), {
  1111 |       timeout: 30_000,
  1112 |     }).toBe(false);
  1113 | 
  1114 |     await zoomGraph(page, 480, 7);
  1115 | 
  1116 |     console.info('Kanban far-zoom', await page.evaluate(() => {
  1117 |       const container = document.getElementById('graph3d-container');
  1118 |       return {
  1119 |         cameraDistance: container?.getAttribute('data-camera-distance'),
  1120 |         rowLabels: Array.from(container?.querySelectorAll('[data-kanban-row-label]') ?? []).map((label) => {
  1121 |           const element = label as HTMLElement;
  1122 |           const rect = element.getBoundingClientRect();
  1123 |           return {
  1124 |             layoutX: element.getAttribute('data-layout-anchor-x'),
  1125 |             layoutY: element.getAttribute('data-layout-anchor-y'),
  1126 |             display: element.style.display,
  1127 |             height: rect.height,
  1128 |             transform: element.style.transform,
  1129 |           };
  1130 |         }),
  1131 |       };
  1132 |     }));
  1133 | 
> 1134 |     await expect.poll(
       |     ^ Error: expect(received).toBeLessThan(expected)
  1135 |       async () => (await visibleGuideMetrics(page, '#graph3d-container [data-kanban-row-label]'))?.height ?? Number.POSITIVE_INFINITY,
  1136 |       { timeout: 30_000 },
  1137 |     ).toBeLessThan(2.5);
  1138 | 
  1139 |     await expect.poll(async () => {
  1140 |       const metrics = await Promise.all(
  1141 |         candidate.stateSamples.map((sample) => graphNodeMetrics(page, sample.id)),
  1142 |       );
  1143 |       return metrics.every((sample) => sample !== null);
  1144 |     }, {
  1145 |       timeout: 30_000,
  1146 |     }).toBe(true);
  1147 | 
  1148 |     const orderedSamples = (await Promise.all(
  1149 |       candidate.stateSamples.map(async (sample) => ({
  1150 |         ...sample,
  1151 |         metrics: await graphNodeMetrics(page, sample.id),
  1152 |       })),
  1153 |     ))
  1154 |       .filter((sample): sample is typeof sample & { metrics: GraphNodeMetrics } => sample.metrics !== null)
  1155 |       .sort((left, right) => {
  1156 |         const rankDelta = kanbanStateRank(left.state) - kanbanStateRank(right.state);
  1157 |         return rankDelta === 0 ? left.state.localeCompare(right.state) : rankDelta;
  1158 |       });
  1159 | 
  1160 |     expect(
  1161 |       orderedSamples.length,
  1162 |       'kanban candidate should provide visible state samples after switching layouts',
  1163 |     ).toBeGreaterThanOrEqual(2);
  1164 | 
  1165 |     for (let index = 1; index < orderedSamples.length; index += 1) {
  1166 |       expect(
  1167 |         orderedSamples[index].metrics.layoutX,
  1168 |         `state ${orderedSamples[index].state} should render to the right of ${orderedSamples[index - 1].state} in kanban mode`,
  1169 |       ).not.toBeNull();
  1170 |       expect(
  1171 |         orderedSamples[index - 1].metrics.layoutX,
  1172 |         `state ${orderedSamples[index - 1].state} should expose a layout x-coordinate in kanban mode`,
  1173 |       ).not.toBeNull();
  1174 |       expect(
  1175 |         orderedSamples[index].metrics.layoutX!,
  1176 |         `state ${orderedSamples[index].state} should render to the right of ${orderedSamples[index - 1].state} in kanban mode`,
  1177 |       ).toBeGreaterThan(orderedSamples[index - 1].metrics.layoutX! + 0.25);
  1178 |     }
  1179 | 
  1180 |     await attachScreenshot(page, testInfo, 'graph-kanban-state-columns');
  1181 |   });
  1182 | });
```
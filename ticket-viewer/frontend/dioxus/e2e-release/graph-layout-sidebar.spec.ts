import { expect, test } from '@playwright/test';
import {
  attachScreenshot,
  findGraphSelectionCandidate,
  findKanbanLayoutCandidate,
  kanbanStateRank,
  openCandidateTicket,
  openTicketById,
} from './graph-detail-candidates';
import {
  GRAPH_ACTIVE_STYLE,
  graphHierarchyMetrics,
  graphNodeMetrics,
  visibleGuideCount,
  visibleGuideMetrics,
  visibleRowLabelOverlapsVisibleNode,
  zoomGraph,
  type GraphNodeMetrics,
} from './graph-detail-interactions';

test.use({
  headless: false,
  launchOptions: {
    args: [
      '--enable-unsafe-webgpu',
      '--enable-features=Vulkan',
      '--use-vulkan=swiftshader',
      '--use-webgpu-adapter=swiftshader',
    ],
  },
});

test.describe('ticket-viewer — graph layout and projection', () => {
  test('graph mode defaults to hierarchical isometric controls and preserves top-to-bottom depth ordering', async ({ page }, testInfo) => {
    test.setTimeout(120_000);

    const candidate = await findGraphSelectionCandidate(page);

    await openCandidateTicket(page, candidate);

    await page.getByRole('button', { name: /^Graph$/ }).first().click();
    await expect(page.locator('#graph3d-container')).toBeVisible({ timeout: 30_000 });

    await page.waitForFunction(
      ({ rootTitle, childTitle }) => {
        const root = document.getElementById('graph3d-container');
        if (!root) {
          return false;
        }
        const text = Array.from(root.querySelectorAll('[data-node-idx]'))
          .filter((node) => (node as HTMLElement).style.display !== 'none')
          .map((node) => node.textContent || '');
        return text.some((value) => value.includes(rootTitle)) &&
          text.some((value) => value.includes(childTitle));
      },
      { rootTitle: candidate.rootTitle, childTitle: candidate.childTitle },
      { timeout: 30_000 },
    );

    const rootNode = page
      .locator('#graph3d-container [data-node-idx]:visible')
      .filter({ hasText: candidate.rootTitle })
      .first();
    const childNode = page
      .locator('#graph3d-container [data-node-idx]:visible')
      .filter({ hasText: candidate.childTitle })
      .first();

    await expect(rootNode).toBeVisible();
    await expect(childNode).toBeVisible();

    const rootBox = await rootNode.boundingBox();
    const childBox = await childNode.boundingBox();
    expect(rootBox, 'root graph node must expose a screen box').not.toBeNull();
    expect(childBox, 'child graph node must expose a screen box').not.toBeNull();
    expect(
      childBox!.y,
      'child node should render below the root in the default hierarchy view',
    ).toBeGreaterThan(rootBox!.y + 10);

    await page.getByTitle('Graph settings').click();
    const hierarchicalButton = page.getByRole('button', { name: 'Hierarchical 3D' });
    const orthographicButton = page.getByRole('button', { name: 'Orthographic' });
    await expect(hierarchicalButton).toBeVisible();
    await expect(orthographicButton).toBeVisible();
    await expect(hierarchicalButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);
    await expect(orthographicButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);

    await attachScreenshot(page, testInfo, 'graph-default-isometric-layout');
  });

  test('graph settings can switch away from and restore the default layout and projection', async ({ page }, testInfo) => {
    test.setTimeout(120_000);

    const candidate = await findGraphSelectionCandidate(page);

    await openCandidateTicket(page, candidate);

    await page.getByRole('button', { name: /^Graph$/ }).first().click();
    await expect(page.locator('#graph3d-container')).toBeVisible({ timeout: 30_000 });

    await expect.poll(() => graphHierarchyMetrics(page, candidate.rootId, candidate.childId), {
      timeout: 30_000,
    }).toMatchObject({
      visibleNodeCount: expect.any(Number),
    });

    const defaultMetrics = await graphHierarchyMetrics(page, candidate.rootId, candidate.childId);
    expect(defaultMetrics, 'default graph layout should expose root and child positions').not.toBeNull();
    expect(
      defaultMetrics!.childY,
      'default hierarchical view should render the child below the root',
    ).toBeGreaterThan(defaultMetrics!.rootY + 10);

    await page.getByTitle('Graph settings').click();

    const hierarchicalButton = page.getByRole('button', { name: 'Hierarchical 3D' });
    const flatButton = page.getByRole('button', { name: 'Flat 2D' });
    const perspectiveButton = page.getByRole('button', { name: 'Perspective' });
    const orthographicButton = page.getByRole('button', { name: 'Orthographic' });

    await expect(hierarchicalButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);
    await expect(orthographicButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);

    await flatButton.click();
    await perspectiveButton.click();

    await expect(flatButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);
    await expect(perspectiveButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);

    const switchedMetrics = await graphHierarchyMetrics(page, candidate.rootId, candidate.childId);
    expect(switchedMetrics, 'graph should stay mounted after switching layout and projection').not.toBeNull();
    expect(
      switchedMetrics!.visibleNodeCount,
      'root and child nodes should remain visible after switching away from defaults',
    ).toBeGreaterThanOrEqual(2);

    await hierarchicalButton.click();
    await orthographicButton.click();

    await expect(hierarchicalButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);
    await expect(orthographicButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);

    await expect.poll(() => graphHierarchyMetrics(page, candidate.rootId, candidate.childId), {
      timeout: 20_000,
    }).toMatchObject({
      visibleNodeCount: expect.any(Number),
    });

    const restoredMetrics = await graphHierarchyMetrics(page, candidate.rootId, candidate.childId);
    expect(restoredMetrics, 'restored default graph layout should expose root and child positions').not.toBeNull();
    expect(
      restoredMetrics!.childY,
      'restoring the default layout should preserve top-to-bottom hierarchy ordering',
    ).toBeGreaterThan(restoredMetrics!.rootY + 10);

    await attachScreenshot(page, testInfo, 'graph-restored-default-layout');
  });

  test('graph settings can switch to kanban state columns', async ({ page }, testInfo) => {
    test.setTimeout(180_000);

    const candidate = await findKanbanLayoutCandidate(page);

    await openTicketById(page, candidate.workspace, candidate.rootId);

    await page.getByRole('button', { name: /^Graph$/ }).first().click();
    await expect(page.locator('#graph3d-container')).toBeVisible({ timeout: 30_000 });

    await page.getByTitle('Graph settings').click();
    const kanbanButton = page.getByRole('button', { name: 'Kanban' });
    await expect(kanbanButton).toBeVisible();
    await kanbanButton.click();
    await expect(kanbanButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);
    await attachScreenshot(page, testInfo, 'graph-kanban-guides-after-switch');

    for (const sample of candidate.stateSamples) {
      await expect.poll(
        () => visibleGuideCount(page, `#graph3d-container [data-kanban-column-header="${sample.state}"]`),
        { timeout: 30_000 },
      ).toBeGreaterThan(0);
    }

    await expect.poll(
      () => visibleGuideCount(page, '#graph3d-container [data-kanban-column-separator]'),
      { timeout: 30_000 },
    ).toBeGreaterThan(0);
    await expect.poll(
      () => visibleGuideCount(page, '#graph3d-container [data-kanban-row-label]'),
      { timeout: 30_000 },
    ).toBeGreaterThan(0);

    await expect.poll(
      async () => (await visibleGuideMetrics(page, '#graph3d-container [data-kanban-row-label]'))?.height ?? 0,
      { timeout: 30_000 },
    ).toBeGreaterThan(0);
    await expect.poll(() => visibleRowLabelOverlapsVisibleNode(page), {
      timeout: 30_000,
    }).toBe(false);

    await zoomGraph(page, -480, 3);

    await expect.poll(
      async () => (await visibleGuideMetrics(page, '#graph3d-container [data-kanban-row-label]'))?.height ?? 0,
      { timeout: 30_000 },
    ).toBeGreaterThan(5.0);

    await expect.poll(() => visibleRowLabelOverlapsVisibleNode(page), {
      timeout: 30_000,
    }).toBe(false);

    await zoomGraph(page, 480, 7);

    await expect.poll(
      async () => (await visibleGuideMetrics(page, '#graph3d-container [data-kanban-row-label]'))?.height ?? Number.POSITIVE_INFINITY,
      { timeout: 30_000 },
    ).toBeLessThan(2.5);

    await expect.poll(async () => {
      const metrics = await Promise.all(
        candidate.stateSamples.map((sample) => graphNodeMetrics(page, sample.id)),
      );
      return metrics.every((sample) => sample !== null);
    }, {
      timeout: 30_000,
    }).toBe(true);

    const orderedSamples = (await Promise.all(
      candidate.stateSamples.map(async (sample) => ({
        ...sample,
        metrics: await graphNodeMetrics(page, sample.id),
      })),
    ))
      .filter((sample): sample is typeof sample & { metrics: GraphNodeMetrics } => sample.metrics !== null)
      .sort((left, right) => {
        const rankDelta = kanbanStateRank(left.state) - kanbanStateRank(right.state);
        return rankDelta === 0 ? left.state.localeCompare(right.state) : rankDelta;
      });

    expect(
      orderedSamples.length,
      'kanban candidate should provide visible state samples after switching layouts',
    ).toBeGreaterThanOrEqual(2);

    for (let index = 1; index < orderedSamples.length; index += 1) {
      expect(
        orderedSamples[index].metrics.layoutX,
        `state ${orderedSamples[index].state} should render to the right of ${orderedSamples[index - 1].state} in kanban mode`,
      ).not.toBeNull();
      expect(
        orderedSamples[index - 1].metrics.layoutX,
        `state ${orderedSamples[index - 1].state} should expose a layout x-coordinate in kanban mode`,
      ).not.toBeNull();
      expect(
        orderedSamples[index].metrics.layoutX!,
        `state ${orderedSamples[index].state} should render to the right of ${orderedSamples[index - 1].state} in kanban mode`,
      ).toBeGreaterThan(orderedSamples[index - 1].metrics.layoutX! + 0.25);
    }

    await attachScreenshot(page, testInfo, 'graph-kanban-state-columns');
  });
});

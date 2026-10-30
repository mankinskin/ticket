import { expect, test } from '@playwright/test';
import {
  attachScreenshot,
  findGraphSelectionCandidate,
  openCandidateTicket,
} from './graph-detail-candidates';
import {
  dragGraphNode,
  graphCameraDistance,
  graphLodMetrics,
  graphNodeLod,
  graphNodeMetrics,
  zoomGraph,
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

test.describe('ticket-viewer — graph selection updates right detail sidebar', () => {

  test('clicking a graph node updates the detail pane without changing the main ticket route', async ({ page }) => {
    test.setTimeout(120_000);

    const candidate = await findGraphSelectionCandidate(page);

    await openCandidateTicket(page, candidate);

    const detailPanel = page.getByTestId('ticket-detail-panel');
    const componentField = page.getByTestId('ticket-detail-field-component');
    await expect(detailPanel).toBeVisible();
    await expect(componentField).toBeVisible();
    await expect(componentField).not.toContainText(candidate.childComponent);

    await page.getByRole('button', { name: /^Graph$/ }).first().click();
    await expect(page.locator('#graph3d-container')).toBeVisible({ timeout: 30_000 });
    await page.waitForFunction(
      (childTitle) => {
        const root = document.getElementById('graph3d-container');
        if (!root) {
          return false;
        }
        return Array.from(root.querySelectorAll('[data-node-idx]')).some((node) => {
          const element = node as HTMLElement;
          return element.style.display !== 'none' && (element.textContent || '').includes(childTitle);
        });
      },
      candidate.childTitle,
      { timeout: 30_000 },
    );

    const childNode = page
      .locator('#graph3d-container [data-node-idx]:visible')
      .filter({ hasText: candidate.childTitle })
      .first();
    await expect(childNode).toBeVisible();
    await childNode.click();

    await page.getByRole('button', { name: /^Split$/ }).first().click();
    await expect(detailPanel).toBeVisible();
    await expect(componentField).toBeVisible();
    await expect(componentField).toContainText(candidate.childComponent, { timeout: 20_000 });
    await expect
      .poll(() => {
        const currentUrl = new URL(page.url());
        const hashParams = new URLSearchParams(currentUrl.hash.slice(1));
        return {
          pathname: currentUrl.pathname,
          ticketId: hashParams.get('ticket-id') ?? hashParams.get('id'),
        };
      })
      .toEqual({
        pathname: `/workspace/${candidate.workspace}`,
        ticketId: candidate.rootId,
      });
  });

  test('graph selection recenters the active node and dims unrelated nodes without replacing the graph', async ({ page }, testInfo) => {
    test.setTimeout(120_000);

    const candidate = await findGraphSelectionCandidate(page);

    await openCandidateTicket(page, candidate);

    const container = page.locator('#graph3d-container');
    await page.getByRole('button', { name: /^Graph$/ }).first().click();
    await expect(container).toBeVisible({ timeout: 30_000 });

    await expect.poll(async () => page.evaluate(({ rootId, childId }) => {
      const container = document.getElementById('graph3d-container');
      if (!container) {
        return false;
      }
      const visibleNodes = Array.from(container.querySelectorAll('[data-node-id]')).filter((node) => {
        const element = node as HTMLElement;
        return element.style.display !== 'none';
      }) as HTMLElement[];
      return visibleNodes.some((node) => node.dataset.nodeId === rootId) &&
        visibleNodes.some((node) => node.dataset.nodeId === childId);
    }, {
      rootId: candidate.rootId,
      childId: candidate.childId,
    }), {
      timeout: 30_000,
    }).toBe(true);

    const childNode = page.locator(`#graph3d-container [data-node-id="${candidate.childId}"]`).first();
    await expect(childNode).toBeVisible();

    const initialDistance = await page.evaluate((childId) => {
      const container = document.getElementById('graph3d-container');
      if (!container) {
        return null;
      }
      const child = Array.from(container.querySelectorAll('[data-node-id]')).find((node) => {
        const element = node as HTMLElement;
        return element.style.display !== 'none' && element.dataset.nodeId === childId;
      }) as HTMLElement | undefined;
      if (!child) {
        return null;
      }

      const containerRect = container.getBoundingClientRect();
      const childRect = child.getBoundingClientRect();
      const containerCx = containerRect.left + containerRect.width / 2;
      const containerCy = containerRect.top + containerRect.height / 2;
      const childCx = childRect.left + childRect.width / 2;
      const childCy = childRect.top + childRect.height / 2;
      return Math.hypot(childCx - containerCx, childCy - containerCy);
    }, candidate.childId);
    expect(initialDistance, 'selected child node must expose an initial distance').not.toBeNull();

    await childNode.click();

    await expect.poll(async () => page.evaluate(({ childId, maxDistance }) => {
      const container = document.getElementById('graph3d-container');
      if (!container) {
        return false;
      }
      const visibleNodes = Array.from(container.querySelectorAll('[data-node-id]')).filter((node) => {
        const element = node as HTMLElement;
        return element.style.display !== 'none';
      }) as HTMLElement[];
      const child = visibleNodes.find((node) => node.dataset.nodeId === childId);
      if (!child) {
        return false;
      }

      const containerRect = container.getBoundingClientRect();
      const childRect = child.getBoundingClientRect();
      const containerCx = containerRect.left + containerRect.width / 2;
      const containerCy = containerRect.top + containerRect.height / 2;
      const childCx = childRect.left + childRect.width / 2;
      const childCy = childRect.top + childRect.height / 2;
      const dimmedCount = visibleNodes.filter((node) => {
        if (node.dataset.nodeId === childId) {
          return false;
        }
        return Number.parseFloat(getComputedStyle(node).opacity || '1') < 0.5;
      }).length;
      const childDistance = Math.hypot(childCx - containerCx, childCy - containerCy);

      return childDistance <= maxDistance && dimmedCount > 0;
    }, {
      childId: candidate.childId,
      maxDistance: Math.max(48, initialDistance! * 0.75),
    }), {
      timeout: 20_000,
    }).toBe(true);

    const focusedMetrics = await page.evaluate((childId) => {
      const container = document.getElementById('graph3d-container');
      if (!container) {
        return null;
      }
      const visibleNodes = Array.from(container.querySelectorAll('[data-node-id]')).filter((node) => {
        const element = node as HTMLElement;
        return element.style.display !== 'none';
      }) as HTMLElement[];
      const child = visibleNodes.find((node) => node.dataset.nodeId === childId);
      if (!child) {
        return null;
      }

      const containerRect = container.getBoundingClientRect();
      const childRect = child.getBoundingClientRect();
      const containerCx = containerRect.left + containerRect.width / 2;
      const containerCy = containerRect.top + containerRect.height / 2;
      const childCx = childRect.left + childRect.width / 2;
      const childCy = childRect.top + childRect.height / 2;
      const dimmedCount = visibleNodes.filter((node) => {
        if (node.dataset.nodeId === childId) {
          return false;
        }
        return Number.parseFloat(getComputedStyle(node).opacity || '1') < 0.5;
      }).length;

      return {
        childDistance: Math.hypot(childCx - containerCx, childCy - containerCy),
        dimmedCount,
      };
    }, candidate.childId);

    expect(focusedMetrics, 'focused graph metrics must be measurable').not.toBeNull();
    expect(
      focusedMetrics!.childDistance,
      'focused node should move closer to the graph center after selection',
    ).toBeLessThanOrEqual(Math.max(48, initialDistance! * 0.75));
    expect(
      focusedMetrics!.dimmedCount,
      'graph selection should dim at least one unrelated visible node',
    ).toBeGreaterThan(0);

    await attachScreenshot(page, testInfo, 'graph-focused-selection');
  });

  test('graph node LOD keeps the active selection rich while collapsing other visible nodes', async ({ page }, testInfo) => {
    test.setTimeout(120_000);

    const candidate = await findGraphSelectionCandidate(page);

    await openCandidateTicket(page, candidate);

    const container = page.locator('#graph3d-container');
    await page.getByRole('button', { name: /^Graph$/ }).first().click();
    await expect(container).toBeVisible({ timeout: 30_000 });

    await expect.poll(() => graphLodMetrics(page, candidate.rootId), {
      timeout: 20_000,
    }).toMatchObject({
      selectedLod: 'rich',
      visibleNodeCount: expect.any(Number),
    });

    const initialMetrics = await graphLodMetrics(page, candidate.rootId);
    expect(initialMetrics, 'root graph node should expose LOD metrics').not.toBeNull();
    expect(
      initialMetrics!.collapsedNodes,
      'workspace graph should collapse at least one non-selected visible node to a smaller LOD tier',
    ).toBeGreaterThan(0);

    await zoomGraph(page, 480, 6);

    await expect.poll(() => graphLodMetrics(page, candidate.rootId), {
      timeout: 20_000,
    }).toMatchObject({
      selectedLod: 'rich',
      visibleNodeCount: expect.any(Number),
    });

    const zoomedOutMetrics = await graphLodMetrics(page, candidate.rootId);
    expect(zoomedOutMetrics, 'zoomed-out root graph node should expose LOD metrics').not.toBeNull();
    expect(
      zoomedOutMetrics!.collapsedNodes,
      'zooming out should keep at least one non-selected visible node in a smaller LOD tier',
    ).toBeGreaterThan(0);

    const childNode = page.locator(`#graph3d-container [data-node-id="${candidate.childId}"]`).first();
    await expect(childNode).toBeVisible();

    const zoomedOutChildLod = await graphNodeLod(page, candidate.childId);
    expect(
      zoomedOutChildLod === 'compact' || zoomedOutChildLod === 'minimal',
      'zoomed-out child node should remain interactive while rendered in a smaller LOD tier',
    ).toBe(true);

    await childNode.click();

    await expect.poll(() => graphLodMetrics(page, candidate.childId), {
      timeout: 20_000,
    }).toMatchObject({
      selectedLod: 'rich',
      visibleNodeCount: expect.any(Number),
    });

    const childMetrics = await graphLodMetrics(page, candidate.childId);
    expect(childMetrics, 'selected child node should expose LOD metrics').not.toBeNull();
    expect(
      childMetrics!.collapsedNodes,
      'after selection, other visible nodes should still use smaller compact or minimal tiers',
    ).toBeGreaterThan(0);

    await zoomGraph(page, -480, 4);

    await expect.poll(() => graphLodMetrics(page, candidate.childId), {
      timeout: 20_000,
    }).toMatchObject({
      selectedLod: 'rich',
      visibleNodeCount: expect.any(Number),
    });

    const zoomedInMetrics = await graphLodMetrics(page, candidate.childId);
    expect(zoomedInMetrics, 'zoomed-in child graph node should expose LOD metrics').not.toBeNull();
    expect(
      zoomedInMetrics!.minimalNodes,
      'zooming back in should not increase the number of minimal visible nodes',
    ).toBeLessThanOrEqual(zoomedOutMetrics!.minimalNodes);

    await attachScreenshot(page, testInfo, 'graph-node-lod-tiers');
  });

  test('dragged graph layout and camera zoom persist when focus changes inside the same graph', async ({ page }, testInfo) => {
    test.setTimeout(120_000);

    const candidate = await findGraphSelectionCandidate(page);

    await openCandidateTicket(page, candidate);

    await page.getByRole('button', { name: /^Graph$/ }).first().click();
    await expect(page.locator('#graph3d-container')).toBeVisible({ timeout: 30_000 });

    await expect.poll(() => graphNodeMetrics(page, candidate.rootId), {
      timeout: 30_000,
    }).not.toBeNull();
    await expect.poll(() => graphNodeMetrics(page, candidate.childId), {
      timeout: 30_000,
    }).not.toBeNull();

    const childBeforeDrag = await graphNodeMetrics(page, candidate.childId);
    expect(childBeforeDrag, 'child graph node should expose drag metrics before movement').not.toBeNull();

    await dragGraphNode(page, candidate.childId, 120, 60);

    await expect.poll(async () => {
      const metrics = await graphNodeMetrics(page, candidate.childId);
      if (!metrics || !childBeforeDrag) {
        return 0;
      }
      return Math.hypot(
        metrics.centerX - childBeforeDrag.centerX,
        metrics.centerY - childBeforeDrag.centerY,
      );
    }, {
      timeout: 20_000,
    }).toBeGreaterThan(40);

    await zoomGraph(page, 480, 4);

    const rootBeforeFocus = await graphNodeMetrics(page, candidate.rootId);
    const childBeforeFocus = await graphNodeMetrics(page, candidate.childId);
    const cameraDistanceBeforeFocus = await graphCameraDistance(page);
    expect(rootBeforeFocus, 'root graph node should remain visible after dragging and zooming').not.toBeNull();
    expect(childBeforeFocus, 'dragged child graph node should remain visible after zooming').not.toBeNull();
    expect(cameraDistanceBeforeFocus, 'graph container should expose current camera distance').not.toBeNull();

    const childNode = page.locator(`#graph3d-container [data-node-id="${candidate.childId}"]`).first();
    await expect(childNode).toBeVisible();
    await childNode.click();

    await expect.poll(() => graphLodMetrics(page, candidate.childId), {
      timeout: 20_000,
    }).toMatchObject({
      selectedLod: 'rich',
      visibleNodeCount: expect.any(Number),
    });

    const rootAfterFocus = await graphNodeMetrics(page, candidate.rootId);
    const childAfterFocus = await graphNodeMetrics(page, candidate.childId);
    const cameraDistanceAfterFocus = await graphCameraDistance(page);
    expect(rootAfterFocus, 'root node should remain visible after focus change').not.toBeNull();
    expect(childAfterFocus, 'dragged child node should remain visible after focus change').not.toBeNull();
    expect(cameraDistanceAfterFocus, 'camera distance should still be exposed after focus change').not.toBeNull();

    const deltaBeforeFocusX = childBeforeFocus!.centerX - rootBeforeFocus!.centerX;
    const deltaBeforeFocusY = childBeforeFocus!.centerY - rootBeforeFocus!.centerY;
    const deltaAfterFocusX = childAfterFocus!.centerX - rootAfterFocus!.centerX;
    const deltaAfterFocusY = childAfterFocus!.centerY - rootAfterFocus!.centerY;

    expect(
      Math.abs(deltaAfterFocusX - deltaBeforeFocusX),
      'same-graph focus changes should preserve the dragged horizontal node offset',
    ).toBeLessThan(24);
    expect(
      Math.abs(deltaAfterFocusY - deltaBeforeFocusY),
      'same-graph focus changes should preserve the dragged vertical node offset',
    ).toBeLessThan(24);
    expect(
      Math.abs(cameraDistanceAfterFocus! - cameraDistanceBeforeFocus!),
      'focus changes should preserve the user-adjusted camera zoom distance',
    ).toBeLessThan(0.25);

    await attachScreenshot(page, testInfo, 'graph-persisted-layout-and-zoom');
  });
});

import { expect, type Page } from '@playwright/test';

export const GRAPH_ACTIVE_STYLE = /#93bbff|79,140,255/;

export interface GraphNodeMetrics {
  x: number;
  y: number;
  width: number;
  height: number;
  centerX: number;
  centerY: number;
  lod: string | null;
  layoutX: number | null;
  layoutY: number | null;
  layoutZ: number | null;
}

export async function graphLodMetrics(
  page: Page,
  selectedId: string,
): Promise<null | {
  selectedLod: string | null;
  visibleNodeCount: number;
  collapsedNodes: number;
  compactNodes: number;
  minimalNodes: number;
}> {
  return page.evaluate((activeNodeId) => {
    const container = document.getElementById('graph3d-container');
    if (!container) {
      return null;
    }

    const visibleNodes = Array.from(container.querySelectorAll('[data-node-id]')).filter((node) => {
      const element = node as HTMLElement;
      return element.style.display !== 'none';
    }) as HTMLElement[];
    const selectedNode = visibleNodes.find((node) => node.dataset.nodeId === activeNodeId);
    if (!selectedNode) {
      return null;
    }

    let compactNodes = 0;
    let minimalNodes = 0;
    let collapsedNodes = 0;
    for (const node of visibleNodes) {
      const lod = node.getAttribute('data-node-lod');
      if (lod === 'compact') {
        compactNodes += 1;
      }
      if (lod === 'minimal') {
        minimalNodes += 1;
      }
      if (node.dataset.nodeId !== activeNodeId && (lod === 'compact' || lod === 'minimal')) {
        collapsedNodes += 1;
      }
    }

    return {
      selectedLod: selectedNode.getAttribute('data-node-lod'),
      visibleNodeCount: visibleNodes.length,
      collapsedNodes,
      compactNodes,
      minimalNodes,
    };
  }, selectedId);
}

export async function graphHierarchyMetrics(
  page: Page,
  rootId: string,
  childId: string,
): Promise<null | {
  visibleNodeCount: number;
  rootX: number;
  rootY: number;
  childX: number;
  childY: number;
}> {
  return page.evaluate(({ activeRootId, activeChildId }) => {
    const container = document.getElementById('graph3d-container');
    if (!container) {
      return null;
    }

    const visibleNodes = Array.from(container.querySelectorAll('[data-node-id]')).filter((node) => {
      const element = node as HTMLElement;
      return element.style.display !== 'none';
    }) as HTMLElement[];
    const rootNode = visibleNodes.find((node) => node.dataset.nodeId === activeRootId);
    const childNode = visibleNodes.find((node) => node.dataset.nodeId === activeChildId);
    if (!rootNode || !childNode) {
      return null;
    }

    const rootRect = rootNode.getBoundingClientRect();
    const childRect = childNode.getBoundingClientRect();
    return {
      visibleNodeCount: visibleNodes.length,
      rootX: rootRect.x,
      rootY: rootRect.y,
      childX: childRect.x,
      childY: childRect.y,
    };
  }, { activeRootId: rootId, activeChildId: childId });
}

export async function graphNodeMetrics(
  page: Page,
  nodeId: string,
): Promise<GraphNodeMetrics | null> {
  return page.evaluate((activeNodeId) => {
    const container = document.getElementById('graph3d-container');
    if (!container) {
      return null;
    }

    const node = Array.from(container.querySelectorAll('[data-node-id]')).find((candidate) => {
      const element = candidate as HTMLElement;
      return element.style.display !== 'none' && element.dataset.nodeId === activeNodeId;
    }) as HTMLElement | undefined;
    if (!node) {
      return null;
    }

    const rect = node.getBoundingClientRect();
    const parseCoordinate = (name: string): number | null => {
      const value = node.getAttribute(name);
      if (!value) {
        return null;
      }
      const parsed = Number.parseFloat(value);
      return Number.isFinite(parsed) ? parsed : null;
    };
    return {
      x: rect.x,
      y: rect.y,
      width: rect.width,
      height: rect.height,
      centerX: rect.x + rect.width / 2,
      centerY: rect.y + rect.height / 2,
      lod: node.getAttribute('data-node-lod'),
      layoutX: parseCoordinate('data-layout-x'),
      layoutY: parseCoordinate('data-layout-y'),
      layoutZ: parseCoordinate('data-layout-z'),
    };
  }, nodeId);
}

export async function graphCameraDistance(page: Page): Promise<number | null> {
  return page.evaluate(() => {
    const container = document.getElementById('graph3d-container');
    if (!container) {
      return null;
    }
    const distance = container.getAttribute('data-camera-distance');
    if (!distance) {
      return null;
    }
    const parsed = Number.parseFloat(distance);
    return Number.isFinite(parsed) ? parsed : null;
  });
}

export async function visibleGuideCount(
  page: Page,
  selector: string,
): Promise<number> {
  return page.locator(selector).evaluateAll((elements) => {
    return elements.filter((element) => {
      const html = element as HTMLElement;
      const style = window.getComputedStyle(html);
      return style.display !== 'none' && style.visibility !== 'hidden' && style.opacity !== '0';
    }).length;
  });
}

export async function visibleGuideMetrics(
  page: Page,
  selector: string,
): Promise<null | {
  width: number;
  height: number;
}> {
  return page.evaluate((activeSelector) => {
    const container = document.getElementById('graph3d-container');
    if (!container) {
      return null;
    }

    const guide = Array.from(container.querySelectorAll(activeSelector)).find((candidate) => {
      const element = candidate as HTMLElement;
      const style = window.getComputedStyle(element);
      return style.display !== 'none' && style.visibility !== 'hidden' && style.opacity !== '0';
    }) as HTMLElement | undefined;
    if (!guide) {
      return null;
    }

    const rect = guide.getBoundingClientRect();
    return {
      width: rect.width,
      height: rect.height,
    };
  }, selector);
}

export async function visibleRowLabelOverlapsVisibleNode(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    const container = document.getElementById('graph3d-container');
    if (!container) {
      return true;
    }

    const isVisible = (candidate: Element): candidate is HTMLElement => {
      const element = candidate as HTMLElement;
      const style = window.getComputedStyle(element);
      return style.display !== 'none' && style.visibility !== 'hidden' && style.opacity !== '0';
    };

    const labels = Array.from(container.querySelectorAll('[data-kanban-row-label]')).filter(isVisible);
    const nodes = Array.from(container.querySelectorAll('[data-node-idx]')).filter(isVisible);

    return labels.some((label) => {
      const labelRect = label.getBoundingClientRect();
      return nodes.some((node) => {
        const nodeRect = node.getBoundingClientRect();
        return labelRect.left < nodeRect.right
          && labelRect.right > nodeRect.left
          && labelRect.top < nodeRect.bottom
          && labelRect.bottom > nodeRect.top;
      });
    });
  });
}

export async function graphNodeLod(
  page: Page,
  nodeId: string,
): Promise<string | null> {
  return page.evaluate((activeNodeId) => {
    const container = document.getElementById('graph3d-container');
    if (!container) {
      return null;
    }

    const node = Array.from(container.querySelectorAll('[data-node-id]')).find((candidate) => {
      const element = candidate as HTMLElement;
      return element.style.display !== 'none' && element.dataset.nodeId === activeNodeId;
    }) as HTMLElement | undefined;
    return node?.getAttribute('data-node-lod') ?? null;
  }, nodeId);
}

export async function zoomGraph(
  page: Page,
  deltaY: number,
  repetitions: number,
): Promise<void> {
  await page.locator('#graph3d-container').hover();
  for (let repeatIndex = 0; repeatIndex < repetitions; repeatIndex += 1) {
    await page.mouse.wheel(0, deltaY);
  }
}

export async function dragGraphNode(
  page: Page,
  nodeId: string,
  deltaX: number,
  deltaY: number,
): Promise<void> {
  const metrics = await graphNodeMetrics(page, nodeId);
  expect(metrics, `graph node ${nodeId} should expose drag metrics`).not.toBeNull();

  await page.mouse.move(metrics!.centerX, metrics!.centerY);
  await page.mouse.down();
  await page.mouse.move(
    metrics!.centerX + deltaX,
    metrics!.centerY + deltaY,
    { steps: 12 },
  );
  await page.mouse.up();
}

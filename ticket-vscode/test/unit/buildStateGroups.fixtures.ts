import { TicketTreeProvider, StateGroupItem, TicketItem } from '../../src/ticketProvider';
import { FilterControlItem } from '../../src/ticketTreeItems';
import type { TicketSummary, EdgeRecord } from '../../src/api';
import type { CoreApi } from '../../src/coreLoader';
import * as api from '../../src/api';

export const SCHEMA_STATES = ['open', 'planned', 'in-implementation', 'in-review', 'done', 'cancelled'];

export function makeTicket(
  id: string,
  state: string,
  title = `Ticket ${id.slice(0, 8)}`,
): TicketSummary {
  return {
    id,
    type: 'tracker-improvement',
    title,
    state,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    fields: {},
  };
}

export function makeEdge(from: string, to: string): EdgeRecord {
  return { from, to, kind: 'depends_on' };
}

export function makeCoreMock(): CoreApi {
  function matchesTicket(ticket: TicketSummary, stateFilter: string, query: string): boolean {
    const state = stateFilter.trim();
    if (state && ticket.state !== state) {
      return false;
    }

    const needle = query.trim().toLowerCase();
    if (!needle) {
      return true;
    }

    return (ticket.title ?? '').toLowerCase().includes(needle)
      || ticket.id.toLowerCase().includes(needle);
  }

  return {
    core_version: () => 'test-core',
    ticket_matches: matchesTicket,
    build_dependency_maps(tickets, edges) {
      const ticketIds = new Set(tickets.map(ticket => ticket.id));
      const depsOf = new Map<string, string[]>();
      const parentOf = new Map<string, string[]>();

      for (const edge of edges) {
        if (!ticketIds.has(edge.from) || !ticketIds.has(edge.to)) {
          continue;
        }

        const deps = depsOf.get(edge.from) ?? [];
        deps.push(edge.to);
        depsOf.set(edge.from, deps);

        const parents = parentOf.get(edge.to) ?? [];
        parents.push(edge.from);
        parentOf.set(edge.to, parents);
      }

      return { depsOf, parentOf };
    },
    build_state_groups(tickets, edges, stateOrder, query) {
      const visibleTickets = tickets.filter(ticket => matchesTicket(ticket, '', query));
      const ticketIds = new Set(visibleTickets.map(ticket => ticket.id));
      const parentOf = new Map<string, string[]>();

      for (const edge of edges) {
        if (!ticketIds.has(edge.from) || !ticketIds.has(edge.to)) {
          continue;
        }

        const parents = parentOf.get(edge.to) ?? [];
        parents.push(edge.from);
        parentOf.set(edge.to, parents);
      }

      const grouped = new Map<string, TicketSummary[]>();
      for (const ticket of visibleTickets) {
        const state = ticket.state ?? 'unknown';
        const bucket = grouped.get(state) ?? [];
        bucket.push(ticket);
        grouped.set(state, bucket);
      }

      const makeGroup = (state: string, bucket: TicketSummary[]) => {
        const stateIds = new Set(bucket.map(ticket => ticket.id));
        const rootIds = bucket
          .filter(ticket => !(parentOf.get(ticket.id) ?? []).some(parentId => stateIds.has(parentId)))
          .map(ticket => ticket.id);
        return { state, total: bucket.length, rootIds };
      };

      const result: Array<{ state: string; total: number; rootIds: string[] }> = [];
      for (const state of stateOrder) {
        const bucket = grouped.get(state);
        if (bucket && bucket.length > 0) {
          result.push(makeGroup(state, bucket));
          grouped.delete(state);
        }
      }
      for (const [state, bucket] of [...grouped.entries()].sort(([a], [b]) => a.localeCompare(b))) {
        if (bucket.length > 0) {
          result.push(makeGroup(state, bucket));
        }
      }
      return result;
    },
    supports_server_control: () => true,
    supports_browser_bridge: () => true,
    supports_file_browsing: () => true,
    ticket_viewer_url: (baseUrl, workspace, ticketId) => `${baseUrl}/workspace/${encodeURIComponent(workspace)}/ticket/${encodeURIComponent(ticketId)}`,
    ticket_display_label: (id, title) => title ?? id,
  };
}

/**
 * Create the provider and wait for the initial load to complete.
 * Injects controlled tickets, edges, and schema via API mocks.
 */
export async function buildProvider(
  tickets: TicketSummary[],
  edges: EdgeRecord[],
): Promise<TicketTreeProvider> {
  const mockApi = api as jest.Mocked<typeof api>;
  mockApi.fetchAllTickets.mockResolvedValue(tickets);
  mockApi.fetchEdges.mockResolvedValue(edges);
  mockApi.fetchSchemas.mockResolvedValue([
    {
      type_id: 'tracker-improvement',
      states: SCHEMA_STATES,
      transitions: [],
      required_states: ['in-review'],
      terminal_states: ['done', 'cancelled'],
    },
  ]);

  const core = makeCoreMock();

  const provider = new TicketTreeProvider(
    'http://localhost:3002',
    'default',
    0, // no auto-refresh
    undefined,
    undefined,
    undefined,
    core,
  );

  // Wait for the async load() to complete by polling the idle state.
  // load() fires onDidChangeTreeData twice: at start (loading) and on finish.
  await new Promise<void>(resolve => {
    const sub = provider.onDidChangeTreeData(() => {
      // The second fire happens when loading is done (idle or error).
      if ((provider as any).state !== 'loading') {
        sub.dispose();
        resolve();
      }
    });
  });

  return provider;
}

export async function waitForProviderReload(
  provider: TicketTreeProvider,
  action: () => void,
): Promise<void> {
  await new Promise<void>(resolve => {
    const sub = provider.onDidChangeTreeData(() => {
      if ((provider as any).state !== 'loading') {
        sub.dispose();
        resolve();
      }
    });
    action();
  });
}

/** Get all root-level state group items. */
export function getRootGroups(provider: TicketTreeProvider): StateGroupItem[] {
  return (provider.getChildren(undefined) as Array<StateGroupItem | FilterControlItem>)
    .filter((item): item is StateGroupItem => item instanceof StateGroupItem);
}

export function getRootControls(provider: TicketTreeProvider): FilterControlItem[] {
  return (provider.getChildren(undefined) as Array<StateGroupItem | FilterControlItem>)
    .filter((item): item is FilterControlItem => item instanceof FilterControlItem);
}

/** Get the state group for the given state, or null. */
export function getGroup(provider: TicketTreeProvider, state: string): StateGroupItem | null {
  return getRootGroups(provider).find(g => g.state === state) ?? null;
}

/** Get the displayed ticket items inside a state group. */
export function getGroupItems(provider: TicketTreeProvider, group: StateGroupItem): TicketItem[] {
  return provider.getChildren(group) as TicketItem[];
}

/** Collect all ticket IDs visible recursively in a state folder (BFS, depth-limited). */
export function collectAllVisibleIds(
  provider: TicketTreeProvider,
  group: StateGroupItem,
  maxDepth = 5,
): Set<string> {
  const result = new Set<string>();
  const queue: Array<{ item: TicketItem; depth: number }> = getGroupItems(provider, group).map(
    item => ({ item, depth: 0 }),
  );
  while (queue.length > 0) {
    const { item, depth } = queue.shift()!;
    result.add(item.ticket.id);
    if (depth < maxDepth) {
      const children = provider.getChildren(item) as TicketItem[];
      for (const child of children) {
        if (child instanceof TicketItem) {
          queue.push({ item: child, depth: depth + 1 });
        }
      }
    }
  }
  return result;
}

// ── IDs for the real-world regression scenario ───────────────────────────────
export const CANCELLED_PARENT_ID = '48ea4df8-25f5-46ce-b2cc-ff00d32ddd47';
export const NEW_CHILD_ID = 'ee43f72e-53ef-4937-8216-92e17f185d85';

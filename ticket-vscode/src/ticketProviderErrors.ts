import type { TicketListFilters } from './api';

export function normalizeFilterValue(value: string | undefined): string | undefined {
  const normalized = value?.trim();
  return normalized ? normalized : undefined;
}

export function isApiRequestError(error: unknown): error is {
  operation: string;
  url: string;
  method: string;
  status?: number;
  responseBody?: string;
  message: string;
} {
  if (!error || typeof error !== 'object') {
    return false;
  }

  const candidate = error as Record<string, unknown>;
  return typeof candidate.operation === 'string'
    && typeof candidate.url === 'string'
    && typeof candidate.method === 'string'
    && typeof candidate.message === 'string';
}

export function formatErrorStateMessage(
  baseUrl: string,
  workspace: string,
  filters: TicketListFilters,
  error: unknown,
): { label: string; tooltip: string } {
  const lines = [
    `Server URL: ${baseUrl}`,
    `Workspace: ${workspace}`,
  ];
  const filterSummary = [
    filters.query ? `query=${filters.query}` : undefined,
    filters.state ? `state=${filters.state}` : undefined,
  ].filter((value): value is string => Boolean(value));
  lines.push(`Filters: ${filterSummary.length > 0 ? filterSummary.join(', ') : 'none'}`);

  if (isApiRequestError(error)) {
    lines.push(`Operation: ${error.operation}`);
    lines.push(`Request: ${error.method} ${error.url}`);
    if (typeof error.status === 'number') {
      lines.push(`HTTP status: ${error.status}`);
    }
    if (error.responseBody && error.responseBody !== '') {
      lines.push(`Response: ${error.responseBody}`);
    }
    const statusSuffix = typeof error.status === 'number' ? ` (HTTP ${error.status})` : '';
    return {
      label: `Request failed${statusSuffix}: ${error.operation}`,
      tooltip: `${lines.join('\n')}\n\n${error.message}\n\nUse the ▶ button to start or reconnect the server task.`,
    };
  }

  const message = error instanceof Error ? error.message : String(error);
  return {
    label: 'Server not reachable',
    tooltip: `${lines.join('\n')}\n\n${message}\n\nUse the ▶ button to start or reconnect the server task.`,
  };
}

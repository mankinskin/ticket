import { registerTracingConsoleSuite } from './shared/tracing-console-suite';
import { registerClientLogApiSuite } from './shared/client-log-api-suite';
import { TICKET_VIEWER } from './shared/viewers';

registerTracingConsoleSuite(TICKET_VIEWER);
registerClientLogApiSuite(TICKET_VIEWER);
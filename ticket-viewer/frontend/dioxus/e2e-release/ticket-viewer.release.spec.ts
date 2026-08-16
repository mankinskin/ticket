import { registerCommonViewerSuite } from './shared/common-viewer-suite';
import { registerDioxusThemeSuite } from './shared/dioxus-theme-suite';
import { TICKET_VIEWER } from './shared/viewers';

registerCommonViewerSuite(TICKET_VIEWER);
registerDioxusThemeSuite(TICKET_VIEWER);
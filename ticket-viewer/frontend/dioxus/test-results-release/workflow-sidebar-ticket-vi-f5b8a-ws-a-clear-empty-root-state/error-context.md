# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: workflow-sidebar.spec.ts >> ticket-viewer — workflow sidebar >> workflow blockers mode keeps browse intact and shows a clear empty-root state
- Location: e2e-release\workflow-sidebar.spec.ts:438:7

# Error details

```
Error: patch ticket state failed for ae490249-6573-4356-955f-eb790d8e03f9 -> ready

expect(received).toBe(expected) // Object.is equality

Expected: true
Received: false
```

# Test source

```ts
  132 |     );
  133 | 
  134 |     let settled = false;
  135 |     let output = '';
  136 |     const timeout = setTimeout(() => {
  137 |       if (!settled) {
  138 |         viewer.kill();
  139 |         reject(new Error(`Timed out waiting for ticket-viewer port announcement. Output:\n${output}`));
  140 |       }
  141 |     }, 60_000);
  142 | 
  143 |     const onData = async (chunk: Buffer) => {
  144 |       output += chunk.toString();
  145 |       const match = output.match(/TICKET_VIEWER_PORT=(\d+)/);
  146 |       if (!match || settled) {
  147 |         return;
  148 |       }
  149 | 
  150 |       settled = true;
  151 |       clearTimeout(timeout);
  152 |       const url = `http://127.0.0.1:${match[1]}`;
  153 |       try {
  154 |         await waitForViewer(url);
  155 |         resolve({ url, viewer });
  156 |       } catch (error) {
  157 |         viewer.kill();
  158 |         reject(error);
  159 |       }
  160 |     };
  161 | 
  162 |     viewer.stdout.on('data', (chunk) => {
  163 |       void onData(chunk);
  164 |     });
  165 |     viewer.stderr.on('data', (chunk) => {
  166 |       output += chunk.toString();
  167 |     });
  168 |     viewer.on('exit', (code) => {
  169 |       if (!settled) {
  170 |         clearTimeout(timeout);
  171 |         reject(new Error(`ticket-viewer exited before startup completed (code ${code}). Output:\n${output}`));
  172 |       }
  173 |     });
  174 |   });
  175 | }
  176 | 
  177 | async function stopSeededViewer(viewer: ChildProcessWithoutNullStreams): Promise<void> {
  178 |   if (viewer.exitCode !== null) {
  179 |     return;
  180 |   }
  181 | 
  182 |   viewer.kill();
  183 |   await new Promise<void>((resolve) => {
  184 |     viewer.once('exit', () => resolve());
  185 |     setTimeout(resolve, 5_000);
  186 |   });
  187 | }
  188 | 
  189 | async function resolveActiveWorkspace(url: string): Promise<string> {
  190 |   const response = await fetch(`${url}/api/workspaces`);
  191 |   expect(response.ok, 'workspace list request failed').toBe(true);
  192 |   const body = (await response.json()) as WorkspacesResponse;
  193 |   const workspace = body.active_workspace || body.workspaces?.[0]?.name;
  194 |   expect(workspace, 'expected an active workspace name').toBeTruthy();
  195 |   return workspace!;
  196 | }
  197 | 
  198 | async function createTicket(
  199 |   page: Page,
  200 |   url: string,
  201 |   workspace: string,
  202 |   title: string,
  203 | ): Promise<string> {
  204 |   const response = await page.request.post(
  205 |     `${url}/api/tickets?workspace=${encodeURIComponent(workspace)}`,
  206 |     {
  207 |       data: {
  208 |         type: 'tracker-improvement',
  209 |         title,
  210 |       },
  211 |     },
  212 |   );
  213 |   const body = await expectJson<CreateTicketResponse>(response, `create ticket failed for ${title}`);
  214 |   const ticketId = body.ticket?.id;
  215 |   expect(ticketId, `expected created ticket id for ${title}`).toBeTruthy();
  216 |   return ticketId!;
  217 | }
  218 | 
  219 | async function patchTicketState(
  220 |   page: Page,
  221 |   url: string,
  222 |   workspace: string,
  223 |   ticketId: string,
  224 |   state: string,
  225 | ): Promise<void> {
  226 |   const response = await page.request.patch(
  227 |     `${url}/api/tickets/${ticketId}?workspace=${encodeURIComponent(workspace)}`,
  228 |     {
  229 |       data: { state },
  230 |     },
  231 |   );
> 232 |   expect(response.ok(), `patch ticket state failed for ${ticketId} -> ${state}`).toBe(true);
      |                                                                                  ^ Error: patch ticket state failed for ae490249-6573-4356-955f-eb790d8e03f9 -> ready
  233 | }
  234 | 
  235 | async function createDependsOnEdge(
  236 |   page: Page,
  237 |   url: string,
  238 |   workspace: string,
  239 |   fromId: string,
  240 |   toId: string,
  241 | ): Promise<void> {
  242 |   const response = await page.request.post(
  243 |     `${url}/api/edges?workspace=${encodeURIComponent(workspace)}`,
  244 |     {
  245 |       data: {
  246 |         from_id: fromId,
  247 |         to_id: toId,
  248 |         kind: 'depends_on',
  249 |       },
  250 |     },
  251 |   );
  252 |   expect(response.ok(), `create edge failed for ${fromId} -> ${toId}`).toBe(true);
  253 | }
  254 | 
  255 | async function seedWorkflowFixture(page: Page): Promise<WorkflowFixture> {
  256 |   const tempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'ticket-viewer-workflow-'));
  257 |   const indexRoot = path.join(tempDir, 'workspace');
  258 |   const blockersRootTitle = 'Workflow blockers root';
  259 |   const directLeafTitle = 'Workflow direct frontier leaf';
  260 |   const nestedParentTitle = 'Workflow nested blocker parent';
  261 |   const nestedLeafTitle = 'Workflow nested frontier leaf';
  262 |   const unblockedByRootTitle = 'Workflow shared prerequisite';
  263 |   const actionableTitle = 'Workflow direct dependent';
  264 |   const extraBlockerTitle = 'Workflow extra blocker';
  265 |   const stillBlockedTitle = 'Workflow still blocked dependent';
  266 |   const transitiveTitle = 'Workflow transitive dependent';
  267 |   await fs.mkdir(indexRoot, { recursive: true });
  268 |   await runTicketCli<unknown>([
  269 |     'init',
  270 |     '--json',
  271 |     '--index-root',
  272 |     indexRoot,
  273 |   ]);
  274 | 
  275 |   const { url, viewer } = await startSeededViewer(indexRoot);
  276 |   const workspace = await resolveActiveWorkspace(url);
  277 | 
  278 |   const blockersRootId = await createTicket(page, url, workspace, blockersRootTitle);
  279 |   const directLeafId = await createTicket(page, url, workspace, directLeafTitle);
  280 |   const nestedParentId = await createTicket(page, url, workspace, nestedParentTitle);
  281 |   const nestedLeafId = await createTicket(page, url, workspace, nestedLeafTitle);
  282 |   const unblockedByRootId = await createTicket(page, url, workspace, unblockedByRootTitle);
  283 |   const actionableId = await createTicket(page, url, workspace, actionableTitle);
  284 |   const extraBlockerId = await createTicket(page, url, workspace, extraBlockerTitle);
  285 |   const stillBlockedId = await createTicket(page, url, workspace, stillBlockedTitle);
  286 |   const transitiveId = await createTicket(page, url, workspace, transitiveTitle);
  287 | 
  288 |   for (const [fromId, toId] of [
  289 |     [blockersRootId, nestedParentId],
  290 |     [blockersRootId, directLeafId],
  291 |     [nestedParentId, nestedLeafId],
  292 |     [actionableId, unblockedByRootId],
  293 |     [stillBlockedId, unblockedByRootId],
  294 |     [stillBlockedId, extraBlockerId],
  295 |     [transitiveId, actionableId],
  296 |   ] as const) {
  297 |     await createDependsOnEdge(page, url, workspace, fromId, toId);
  298 |   }
  299 | 
  300 |   await patchTicketState(page, url, workspace, directLeafId, 'ready');
  301 |   await patchTicketState(page, url, workspace, nestedLeafId, 'ready');
  302 |   await patchTicketState(page, url, workspace, unblockedByRootId, 'ready');
  303 | 
  304 |   return {
  305 |     tempDir,
  306 |     indexRoot,
  307 |     url,
  308 |     viewer,
  309 |     workspace,
  310 |     blockersRootId,
  311 |     blockersRootTitle,
  312 |     unblockedByRootId,
  313 |     unblockedByRootTitle,
  314 |   };
  315 | }
  316 | 
  317 | async function fetchWorkflowNext(
  318 |   page: Page,
  319 |   fixture: WorkflowFixture,
  320 | ): Promise<WorkflowNextResponse> {
  321 |   const params = new URLSearchParams({
  322 |     workspace: fixture.workspace,
  323 |     limit: '50',
  324 |   });
  325 |   const response = await page.request.get(
  326 |     `${fixture.url}/api/workflow/next?${params.toString()}`,
  327 |   );
  328 |   return await expectJson<WorkflowNextResponse>(response, 'workflow next request failed');
  329 | }
  330 | 
  331 | async function fetchWorkflowTree(
  332 |   page: Page,
```
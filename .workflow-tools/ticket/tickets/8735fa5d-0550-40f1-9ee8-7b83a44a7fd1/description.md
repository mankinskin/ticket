Auto-start in `ticket-vscode` should prefer the `ticket-viewer` executable on `PATH` before falling back to a workspace-local `target/debug/ticket-viewer(.exe)` binary. This keeps the extension aligned with the installed managed viewer by default while still preserving an explicit `ticketViewer.serverBinaryPath` override and a debug-binary fallback when no PATH install is available.

Acceptance criteria:
1. `ticketViewer.serverBinaryPath` remains the highest-priority override.
2. With an empty `serverBinaryPath`, auto-detect resolves `ticket-viewer(.exe)` from `PATH` before checking workspace `target/debug/`.
3. If no PATH binary is available, the extension still falls back to the workspace debug binary when present.
4. Extension spec/docs describing `serverBinaryPath` auto-detect order are updated to match the implementation.
5. A focused unit or compile validation covers the new resolution order.

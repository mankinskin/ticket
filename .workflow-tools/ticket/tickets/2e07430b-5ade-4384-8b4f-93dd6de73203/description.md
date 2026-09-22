## Problem

`./target/debug/ticket.exe list --toon | head -40` panics when `head` closes stdout, exits with code 130, and produces no usable output. The same outcome occurs with ordinary early-closing consumers such as `less` and `tail`. The broken-pipe panic caused three separate agent dispatches to abort.

Pipelines with early-closing consumers are standard shell usage. The ticket CLI should treat a broken pipe as normal termination, using standard SIGPIPE behavior or an equivalent clean exit code 0 with truncated output.

## Required State

Handle the broken-pipe condition in the ticket CLI so early stdout closure does not panic and exits 0. Verify the original reproduction command and check whether the same defect affects the `spec`, `audit`, `rule`, and `session` repository CLIs; record each result in the ticket notes.

Related public-ticket-crate context: ticket `ba4aaa9c`.

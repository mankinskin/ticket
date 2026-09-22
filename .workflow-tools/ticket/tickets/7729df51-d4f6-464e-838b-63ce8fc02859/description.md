## Problem

Ticket `e42d8e0a-c210-4efe-a22c-2565079e67b8` (memory-viewers/.ticket) carried the field `deleted: true` in its manifest yet still appeared in `list_tickets` output and resolved normally through `get_ticket`. Either the deletion path is not completing, or `deleted` is a stale/unhandled field that the index and listing layers ignore.

Found incidentally while triaging duplicate tickets. The ticket has since been cancelled, but the underlying storage/index behaviour is unexplained.

## Scope

Investigate and fix. Determine whether `deleted` is a supported field at all; if it is, listing and search must honour it. If it is not, identify what wrote it and whether other tickets carry it.
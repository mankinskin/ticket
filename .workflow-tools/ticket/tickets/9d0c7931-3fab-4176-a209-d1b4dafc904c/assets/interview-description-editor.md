# Interview: Description Editor

**Date:** 2026-04-08
**Applies to:** `9d0c7931` (Description editor)

## Question

What level of complexity for the description editor?

- Plain `<textarea>` + live preview panel (simplest, like GitHub issues)
- CodeMirror-style editor via web-sys wrapping (syntax highlighting in the textarea)
- Rich WYSIWYG (most complex, least common in dev tools)

## Answer

**Textarea element, but we reuse the same text rendering. The textarea only adds the text interaction capabilities.**

## Implications

- Use a plain `<textarea>` for input/editing
- Render the preview using the same `pulldown-cmark` + `syntect` pipeline used in TicketContent
- The textarea and preview share the same rendering path — preview is not a separate system
- No CodeMirror or WYSIWYG — keep it simple
- Split-pane or tabbed layout: edit on left/top, preview on right/bottom
- The textarea handles cursor, selection, undo/redo natively
- Consider live-as-you-type preview with debounced rendering

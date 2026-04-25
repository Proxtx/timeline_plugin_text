# Migration: MongoDB → new SQLite

## Old layout (MongoDB)

Stored text events in the shared `events` collection:

```json
{ "plugin": "timeline_plugin_text",
  "id": "<utc_millis>",
  "timing": [<start_nanos>, <end_nanos>] | [<utc_nanos>],
  "event": "<the actual text>"
}
```

`event` was just a bare string (the text content).

## New layout

Per-plugin data dir (default `./data/plugins/timeline_plugin_text/`):

### `events.db` (SQLite)

| column | value |
|---|---|
| `id` | unchanged from old `id` (utc_millis as a string) |
| `start_ts`, `end_ts` | from `timing[0]/1_000_000`, `timing[1]/1_000_000` (or both equal for instant timing) |
| `title` | `"Text"` (constant) |
| `data` | JSON `{ "text": "<the actual text>" }` |

That's it. The plugin synthesizes "Write Text" placeholder events one
per hour at query time, so there's nothing to migrate for those.

## Per-row conversion

For each `{plugin: "timeline_plugin_text"}` row:

1. `id = old_id` (drop unchanged).
2. `start_ts = timing[0] / 1_000_000`, `end_ts = (timing[1] or timing[0]) / 1_000_000`.
3. `title = "Text"`.
4. `data = json!({ text: event })`.

## Notes

- Idempotent: SQLite primary key uses the original utc_millis id, so
  re-running the import overwrites without duplicating.
- Validate: `SELECT COUNT(*) FROM events` should equal the count of old
  `plugin = "timeline_plugin_text"` rows.

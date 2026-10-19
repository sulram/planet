---
date: 2026-10-19
lives_in:
  - PLUGINS.md#what-the-host-offers
  - /apps/web/src/lib/plugins/plugin.ts
---

# A file goes up through the seam, and no plugin holds the key

**Decision:** A plugin's layer sends a file to the world through its `Seam`: `upload(file)`
resolves to the asset's name, `asset://<sha256>.<ext>`, or throws the
world's refusal code. The page holds the session's key and the seam carries
it; no layer, no kind's component and no plugin is handed the key.

**Why the seam.** Two sessions drew it two ways while each added a kind of
item that is a file: the `.vox` session gave the seam an `upload`, the
images session passed the key down as a prop of every layer, and every kind
posted for itself. One way is kept, the seam's: the key is the page's, as
the socket's URL is, and what crosses to a plugin is a command, an event or
a reading (93); a key in a plugin's hands is a credential it never needs,
and a prop threaded through three components for one call is the kind of
plumbing a stranger's plugin would get wrong.

**Rejected:** the key as a prop of `LayerProps` and `KindProps`, with each
kind calling `$lib/files` itself; a module level store of the key, which
every layer could read.

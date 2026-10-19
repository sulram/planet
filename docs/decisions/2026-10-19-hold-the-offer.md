---
date: 2026-10-19
lives_in:
  - RENDER.md#avatars
---

# The first V holds the whole offer loaded

**Decision:** The first press of V holds every avatar the manifest offers: the wardrobe fetches them one by one, in the order of pressing from the one after next, each only once nothing else is on its way, and keeps them loaded though nobody wears them. The one wait is the first; every press after lands at once. Only the offer is held: a person's own avatar is fetched when worn and dropped when not.

**Why:** An avatar file weighs one to four megabytes and the press is a browse: the person wants to see them all, and without the hold each press is a wait of its own. Ten held avatars cost about fifty megabytes of texels, a price a phone pays once, and only when someone browses.

**Rejected:** Fetch the whole offer at once: on a slow link the nine files share the band with the one the person waits for, and the first wait grows four times (two seconds to nine at 2 MB/s). Fetch only the next one in the cycle: bounded memory, but a quick second press still waits. Warm the browser's cache from the shell: no engine memory, but the engine still parses and uploads on each press, and the desktop reads disk for nothing; it also trusts a cache the seam does not promise. Hold from the start, before any press: pays the price for everyone, browsing or not.

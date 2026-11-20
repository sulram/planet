---
date: 2026-11-20
lives_in:
  - PLUGINS.md#the-core-s-words
  - ARCHITECTURE.md#clients-and-ui
  - GLOSSARY.md
  - /proto/planet/parts/v1/parts.proto
  - /proto/planet/v1/world.proto
  - /crates/world/src/parts.rs
  - /crates/client/src/parts.rs
  - /crates/client/src/lib.rs
  - /plugins/items
  - /apps/web/src/lib/engine/EngineView.svelte
amends:
  - 2026-10-19-116-a-landmark-is-a-shape-of-part-with-a-name
---

# A home and favourites head the chart: an account's are the world's, a visitor's the browser's

**Decision:** A person marks landmarks of the chart as favourites, and one as the home, where the body is born when the page opens with no place. An account's are the world's: the parts owner keeps them under the account's sub, one row an account, and two ops at signed_in, favour and home, set them through the one command and event seam (94). Each charted answer orders the landmarks home first, then the favourites, then the rest, each the nearest the asker first, and says which are which: the chart stays one answer and one poll. A visitor, who has no account, keeps them in the browser: the page hands them to the engine with set_choices, the engine orders its chart by them and says each change with choices, for the page to keep. Welcome says whether the session is a visitor, since a session under bun run dev is an admin with no account. The engine asks the chart on every welcome, and the first answer says where the body is born: home, with the star's place code, or none. A page that asked no place goes there through go_to, the veil down until the ground there is drawn; a place in the address wins, and changing the home moves no body. The search is the page's: by name, case and accent insensitive, over the 256 the world answers.

**Why:** A home is the person's, the same on a phone and a laptop (Marlus, 2026-11-20), and inside a world the world is what knows the account; mundos keeps who someone is, not what they chose in each world. Kept by the parts owner, the choices sit beside the landmarks they name: a landmark removed leaves the chart and the row is pruned at the next choice. The home reaching the page in the first chart keeps the server out of a store only the module reads, and asks no second question.

**Rejected:** The welcome carrying the home: the Go server would read the parts owner's store, which only the module reads (99). A route the page asks before it opens the world: a second question and a second check of who is asking. An account's choices in the browser too: a second device starts over. Search on the server: the world answers 256, and a world past that is an open question.

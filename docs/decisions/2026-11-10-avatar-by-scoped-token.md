---
date: 2026-11-10
lives_in:
  - ROADMAP.md#avatars-through-mundos
  - ARCHITECTURE.md#identity-and-levels
resolves:
  - avatar-claim-and-the-way-back
---

# A VRM worn inside a world reaches mundos by a scoped token, with no reload

**Decision:** mundos owns the avatars (its decisions avatars-live-in-mundos and the two beside it); a world wears what the entry token says in its avatar claim, the VRM's public URL on mundos's CDN, or its own default when the claim is absent. The way back is Marlus's choice in mundos, decision 2026-11-10-avatar-scoped-token: at /enter mundos hands the page, in the fragment beside the identity, a second token signed with the same key, aud mundos, scope avatar, good for a visit; the page keeps it in memory as it keeps the key and never sends it to the world's server. To wear a VRM picked or dropped, the page calls POST /api/me/avatar on PUBLIC_MUNDOS_URL with that token as Bearer and the VRM's URL, a world asset address or a catalog id; mundos copies the file under the account, sets it current and answers 200; then the page tells its own server the new URL, the server tells the others, and the next load's token carries the same. mundos pushes nothing into a running world. A picker inside the world reads mundos's public catalog endpoint with no session. The field names of the fragment and of the body arrive with mundos's item Own VRMs and the way back.

**Why:** A world holds no identity and no list, so what a person picks inside it has to land in their mundos account; a world's token lives a minute with the world as its audience, so a second, scoped token is the one credential that may cross, and it crosses only through the page, never the server.

**Rejected:** A whole-page round trip to a mundos route that ends in /enter: no new auth, but a reload of the world at each change, which hurts in a workshop. A world writing into mundos as the person with the entry token: a credential the world never needs.

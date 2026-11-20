---
date: 2026-11-20
lives_in:
  - /apps/web/src/lib/engine/Stage.svelte
  - ARCHITECTURE.md
amends:
  - 2026-09-28-073-the-address-bar-is-honoured-each-time-a
---

# The address bar's hand is the browser's hashchange, never page.url

**Decision:** The page reads the address bar's place from the browser alone: location.hash when the stage mounts, the link that brought you in, and the browser's hashchange after that, a place pasted over the bar. SvelteKit's page.url is not read for it. What the page writes there as you move stays the engine's: the page's replaceState fires no hashchange, and neither does a rerun of the loads.

**Why:** A language switch sent the body back to the link's place. invalidateAll reruns every load, and SvelteKit hands page.url the URL of the last navigation, the link's hash, long after the page wrote newer poses with replaceState; the stage read that hash as a hand's and walked there (073 took page.url for exactly what a hand put there, which holds only until a load reruns). The browser's hashchange fires for a hand alone: a paste, a typed hash, the back button; never for replaceState, never for a load. The same place pasted twice is still two requests, since the second paste changes the hash the page had written since.

**Rejected:** Comparing the stale hash against every pose the page ever wrote, which grows without bound and turns a real paste of an earlier place into a no-op. Keeping page.url and skipping arrivals while a load reruns, which guesses at SvelteKit's timing. Writing the pose through goto or pushState so SvelteKit's own URL follows, which makes every step a navigation and a history entry the back button would walk.

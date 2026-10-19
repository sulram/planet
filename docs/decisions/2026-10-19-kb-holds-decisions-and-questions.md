---
date: 2026-10-19
lives_in:
  - /AGENTS.md
  - decisions/index.md
  - questions/index.md
  - /kb.config.ts
  - /scripts/kb-plugins.ts
amends:
  - 2026-09-24-067-the-decisions-log-is-one-file-per-entry
  - 2026-09-24-064-a-fact-has-one-home-docs-speak-in-the
---

# Decisions and open questions are kb's, from the studio's registry

**Decision:** The docs keep their homes, and kb (`@tekne/kb`, the studio's own
package on `npm.tekne.studio`) holds their shape: a decision is one file,
`YYYY-MM-DD-slug.md`, with its state's homes in frontmatter; an open question
is one file with its group, owner and what it unblocks, in `docs/questions/`;
every list is built when read, never committed; the shared rules of the
studio's projects are one block in `AGENTS.md`, written by `bun kb gen`, and
`CLAUDE.md` is a symlink to it. `bun run docs` is `kb check`: kb's rules and
this repo's own, in `scripts/kb-plugins.ts`. A decision is cited by a fragment
of its id, the number the first 115 entries keep in their slugs or a piece of
the slug, and the fragment names exactly one file.

**Why:** Three projects of the studio kept decisions, questions and agent
rules in three shapes, and the rules every agent follows were copied by hand.
kb is the shape two of them already have; planet's numbered log was the
third. A running number collides between worktrees, and the index it needed
was one file every branch rewrote; kb's ids carry a date and a slug, and the
list is read, not kept. `kb why` and `kb search` give an agent the decisions
behind a doc before it changes it, which the log alone did not. One package
on the studio's registry, versioned, replaces a copy in each repo that
drifted.

**Rejected:** a copy of kb in `tools/kb`, as the other two projects hold it
today: the copies had already drifted. Dropping the numbers from the first
entries' ids: some nine hundred citations read `(81)` and stay readable.
Numbering new entries: the collision the dates avoid; a new entry takes a
short slug. A VitePress site for the docs: nothing reads it yet; the CLI is
what the agents use. Easing kb's rule on rewriting a decision: planet keeps
its own, said in AGENTS.md: before a release carries it, a decision is clay.

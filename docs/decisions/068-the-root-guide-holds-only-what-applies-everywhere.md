# 68. The root guide holds only what applies everywhere, under 200 lines (decided)

Logged 2026-09-24.

CLAUDE.md enters every session's context, and it had reached 310 lines: rules
carrying their own reasoning, lists a script already owns, and folder rules
every session paid for. It now holds only what applies everywhere and stays
under 200 lines, which `bun run docs` enforces as it does for any doc.

**A rule is one line; its why lives in the decision that made it.** The
docs, writing and reusability sections lost their explanations, which are in
64, 65 and 67. A reader who wants the argument follows the number.

**Folder rules live in the folder.** `apps/web/CLAUDE.md` holds the web rules
and the web pins; `server/CLAUDE.md` the PocketBase pin and the server
layout. Each has an `AGENTS.md` symlink, as the root does, so Claude Code,
Codex and Kimi all read it when working there. The refs licence list is
`docs/REFS.md`, read when a ref is opened and not before.

**What a script owns is not repeated.** The checks `bun run docs` runs are
listed by `scripts/docs.ts`; the allowed dependency graph is its `ALLOWED`
table; the root says where they are.

Rejected: keeping one file and trimming words, which was tried this morning
and gave 231 lines with the folder rules still in it; a generated table of
contents, which adds lines to the file it is meant to shrink.

**Lives in:** CLAUDE.md; `apps/web/CLAUDE.md`; `server/CLAUDE.md`; `docs/REFS.md`.

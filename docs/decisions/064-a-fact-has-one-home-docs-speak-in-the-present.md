# 64. A fact has one home, docs speak in the present, and a script reads them (decided)

Logged 2026-09-24.

Plataforma ITS (`refs/its-plataforma`, branch `staging`) went through the same
cleanup a week earlier and wrote down what it cost: one decision reaching five
edits, two roadmaps crossing, comments telling history that was wrong at the
next change, and 86 broken links found only by a sweep. Its rules are adopted
here, in English, where they fit a docs set an LLM reads every session.

**One home per kind of information**, as a table in CLAUDE.md: state in the
theme doc, why here, next in ROADMAP, open in OPEN, line by line in git. A
change touches at most three of them. This was implicit and was not held: OPEN
carried a fifteen line analysis of sector sizes that 49 and 50 had settled, and
a band question that 49 answers in its fourth paragraph.

**Docs and comments in the present, in the affirmative.** The past is git and
this file. What this retires from the operational docs is the narrative after
a struck item ("twice superseded: first by, then by") and the bullets for
things that do not exist ("deleted with 58; written again when"). A struck item
keeps its pointer and loses its story.

**An entry here ends with Rejected and Lives in**, and a retired entry has its
title struck with its successor's number, so a reader arriving by number sees
at once that it no longer holds. 48 is the first: 58 retires it and said so
only in 58.

**OPEN is tables with an "unblocks" column.** A question nobody waits on is a
wish, and wishes live in ROADMAP. The rows 49 and 50 settled (sector size, the
band following the surface) are gone; the rest are grouped by who or what they
block.

**`bun run docs` checks what a script can check**: every `DECISIONS N` in docs
and code names an entry, every crate bullet in CLAUDE.md is a directory and
every directory has a bullet, every `bun run` name in the docs is a script and
every script is in CLAUDE.md, no em dash anywhere, no operational doc past 200
lines. It runs in `bun run check` and in CI. The `voxel` bullet in CLAUDE.md,
for a crate 58 deleted, is what it would have caught.

Rejected: the ITS format of four to six lines per decision, because this file
carries the reasoning the operational docs are forbidden to hold, and the short
form is the title plus the Lives in line; newest entry at the top, because
entries are cited by number and appending keeps them stable; generated
snippets inside the docs, which ITS uses for its state machine, because nothing
here is yet a table that only copies code; per folder instruction files, which
ITS keeps for its app, domain, mails and PocketBase, not yet: the web app
rules in CLAUDE.md are the first candidate, and when they go they move rather
than being copied.

**Lives in:** CLAUDE.md § Docs style and § Writing; `scripts/docs.ts`.

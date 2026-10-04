# docs: rules

The root `CLAUDE.md` applies here too.

## Test first, write after

- A step is built, shown running and tried by Marlus in the world before its
  docs are written. While it is refined, the commit's why is its record.
- In the commit that changes code: what a script checks (`bun run docs`), and
  the deletion of every line the change made false.
- When the shape has held: the theme doc, once, and a decision only for what
  is dear to undo.

## Homes

| Information | Home |
|---|---|
| Current state | the theme doc: ARCHITECTURE, WORLD, RENDER, PLUGINS, DEPLOY |
| Why, and what was rejected | DECISIONS |
| What comes next, in what order | ROADMAP |
| A question with no answer, a problem with no chosen fix | OPEN |
| A design drawn ahead of its code | a sketch in `sketches/` |
| What changed, line by line | git |

- A change touches at most three homes: the theme doc, DECISIONS, and ROADMAP
  **or** OPEN. Anywhere else, a pointer.
- Operational docs and the guides: titles + bullets, no prose, under 200 lines.
  VISION, DECISIONS and sketches keep prose: they carry the why and the what if.
- OPEN is tables: question, what it unblocks, context. A question nothing
  waits on is a wish, and wishes live in ROADMAP.
- A struck ROADMAP item carries a DECISIONS number, never the story.
- GLOSSARY updates in the change that adds, renames or shifts a term.

## Decisions

- A decision is the next file in `decisions/`: `# NN. Title (status)`,
  `Logged <date>.`, the decision in one breath, the why, **Rejected**,
  **Lives in**. Then `bun run docs gen`.
- It is logged for what is dear to undo, or for a rejection worth keeping.
  The rest is the commit's why.
- Until a release carries what it decides, a decision is clay: rewritten in
  place as the design moves. Released, it is retired by a struck title with
  the number that replaced it, or removed whole by a `**Removes:**` line.

## Sketches

- A sketch draws what may come, whole and far: where the seams lead, what the
  next plugins ask of the host. One file a subject, `sketches/<subject>.md`.
- Its language is prose, the conditional, and ways side by side. Nothing in
  it is decided or built, and no code cites it.
- A piece leaves a sketch for DECISIONS when a campaign pulls it. A sketch
  with nothing left to say is deleted.

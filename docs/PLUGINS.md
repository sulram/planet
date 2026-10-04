# PLUGINS

How the core and a plugin are cut, what each owns and how they speak. The why
is in DECISIONS 88, 91, 93 and 94; the order they are made in is in BRIEF.md.

## The core and a plugin

- The core (88) is what a plugin stands on, and it runs with every plugin
  off. What it is made of: ARCHITECTURE.md § The core and its plugins.
- A plugin is a slice through up to four places: a crate in the client, a
  package in the server, a payload on the wire, a panel in each front end.
  Ours, compiled in: a config at the root lists them and the build bundles
  them, so a version is the core plus the plugins chosen for it.
- Native plugins: chat, building, land, avatars, made in that order (96).
  Chat, building and avatars are extracted from where they stand today.
- Which plugins are on is the world's own (91). The config says whether each
  starts on; the admin switches any of them at the founding and after, in the
  world folder. A plugin switched off keeps its store untouched.
- A plugin imports the core, never another plugin, and draws through `scene`
  as the client does. `ALLOWED` in `scripts/docs.ts` holds it: a plugin's row
  names crates of the core.

## Owners

- Every piece of a world's state has one owner, the core or one plugin (93).
  The owner alone changes it; everyone else sends it an op or asks it a
  question.
- The core owns the recipe, who a session is, its level and its stance.
- A plugin owns what it keeps: its store in the world folder, its payloads on
  the wire, its panel.

## Three conversations

| Conversation | What it does | What may be done with it |
|---|---|---|
| Op | asks an owner to change what it holds; answers that it landed, or the code of why it was refused | checked, logged, taken back |
| Question | asks for an answer and changes nothing | asked again, kept |
| Event | says what happened, to whoever listens | let pass |

- A question is a hook when the core asks its plugins, and a reading when
  anyone asks an owner what it holds.
- A hook has a default answer. A plugin over it answers in its place, with
  the default to fall back on: the level answers who may do what and where,
  and land answers over it.
- A message is one of the three. What is two of them is two messages.
- At the seam they are `client::Command` and `client::Event`, JSON tagged by
  `type`. On the wire they ride the envelope, a plugin's name beside its
  payload. A refusal is a code at both, never a sentence.

## The path of a change

1. An op arrives: from a hand's tool, a macro or an agent.
2. The host asks the permission hook with who, what and where, before the
   owner sees the op.
3. The owner applies its rule.
4. The owner keeps the result and logs the op.
5. An event tells every session.

- The rule is written once: the client predicts with it and the server
  decides with it. How the server runs it: OPEN.md.
- A constraint lives in the rule. One held by a panel alone is one an agent
  and a macro walk past.

## The core's words

- Two plugins meet in three words of the core, and neither names the other.
- **Who**: the account, as mundos signs it, and its level. A person or an
  agent.
- **Where**: the address. A box of it, or a plot, which is the address less
  six bits (77).
- **What**: the name of the action, the plugin's before it: `build.create`.
- A volume is the unit building and land share (95): cells and their history
  are kept by volume, permission is given by volume, and a rollback restores
  a volume.

## What the host offers

- Cut as each plugin asks for it, by extracting what exists (88): commands
  and events on the seam; an envelope on the wire; a registry and its config;
  a place for a panel; a turn in each frame; solids for the footing; plain
  data for the picture through `scene`; the permission hook; a store in the
  world folder; files.
- A hook is written when a plugin asks: who may do what and where, which
  avatars are offered.

## A person and an agent

- What a world offers, it offers to a person and to an agent alike (94).
- A capability is a command at the seam, named, its parameters said. A tool
  is how a hand composes one; an agent sends the command itself.
- Every op a tool makes has a command that asks for it by its parameters.
- Whatever a picture shows has a reading that says the same as data: the
  ground, the cells in a box, who is near, where an account may build.
- An agent is an account as a person is: a door, a level, permissions.

# server: rules

The root `CLAUDE.md` applies here too.

- One Go binary is an instance, and an instance is one world (DECISIONS 87,
  90). `cmd/planet` is the entry; who the instance is comes from the
  environment (docs/DEPLOY.md).
- `internal/world` is the core: the hub, the actor, a session, the recipe, the
  keys and the host of plugins (`plugin.go`). It is told who a session is and
  what the recipe is, through small types of its own.
- `internal/api` is the one package that knows HTTP, mundos's token and the
  world folder at once: the routes, the socket, the front end's files. A route
  answers an error as a code, never a sentence.
- `internal/mundos` checks what mundos signs.
  `internal/folder` is the world folder, and how it is copied into a world's
  next generation (`planet copy`).
- No plugin's code is written here, and no plugin's schema (DECISIONS 99). A
  plugin's server logic goes in its world half, Rust run in the module, and
  the server offers it services that carry no feature.
- `internal/module` runs the module through wazero and is the one package
  that knows it is WASM (DECISIONS 102). It embeds `world.wasm`, which
  `bun run module` builds: run it before `go build`, `go vet` or `go test`.
  A service a plugin asks for is a message in `proto/planet/module/v1` and a
  method of `world.Room`.
- The dependencies are the standard library, the socket, protobuf, wazero
  and SQLite in Go, with no C (DECISIONS 110). Another one is a decision
  before it is an import.
- `internal/store` is what owners keep: a SQLite file apiece in the world
  folder, rows of a key and a value the server never reads.
- `internal/protocol`, the world's wire and the module's bridge, is generated
  by `bun run proto` and committed. Never edited by hand.

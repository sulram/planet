# DEPLOY

How a world is hosted: by mundos (87), from one image, with one folder. What
mundos is and does lives in its own `docs/BRIEFING.md`; this is the instance
type's side of that contract. A line marked (p) is proposed and not yet
confirmed. What of it is built: BRIEF.md.

## What the server carries

- Rendering, terrain generation and collision run in the browser. The server
  never generates a patch.
- The world actor relays small protobuf frames. Twenty people in one world is
  20 x 19 x 15 frames a second, under 1 MB/s.
- The heavy bytes are static. The WASM engine (a few MB) and a field (25 MB,
  16 over the wire as brotli) are kept by the browser for good, since their
  URLs carry a content hash (72); an avatar VRM is revalidated.
- mundos caps each container's memory and CPU. Grow a cap only when a
  measurement says so.

## The image

- One image for each version: `ghcr.io/sulram/planet:<version>`, the version
  its git tag names (21). mundos lists what the registry publishes, pulls the
  version a superadmin asks for, and a world runs only a version, never a
  branch or a hash. The repository and its package are public, so the box
  holds no registry key, as with the Hyperfy fork.
- In it: the Go binary, the web front end's files, the engine's WASM, and the
  default asset set with the baked fields. One process runs (90).
- An `image` script builds it on this machine, for amd64 and arm64, with the
  fields baked here, and pushes it (p). The fields are too large for the
  repository, so the image is built where they are (OPEN.md).
- The Go binary has no CGO, and the engine and the page are the same on every
  architecture: only the last stage of the build differs.

## What mundos sets

| Variable | What |
|---|---|
| `PORT` | where the server listens inside the container |
| `MUNDOS_PUBLIC_KEY` | the Ed25519 public key that checks tokens: PEM, its newlines escaped |
| `PUBLIC_MUNDOS_URL` | mundos's origin, where the door is |
| `PUBLIC_MUNDOS_WORLD` | the world's name, the token's audience |
| `ASSETS_S3_URI`, `ASSETS_BASE_URL` | the generation's bucket folder, and the address browsers read it at |

- The names are the ones the Hyperfy fork reads, so mundos says the same thing
  to every instance type. The bucket's two are absent on a machine without
  one, and the world then keeps its files in its folder (89).
- The port is published on `127.0.0.1` alone. mundos's Caddy is the one thing
  that reaches it, with the world's address and its certificate, and it
  carries the socket with no extra setting.

## The world folder

- A Docker volume, named by mundos after the generation's id and mounted at
  `/world` (p). In it: the recipe and what the admin set, and each plugin's
  SQLite file (89).
- It is the whole world. Nothing of a world lives in the image, and deleting a
  generation deletes its folder and its bucket folder.

## Health

- `GET /api/health` answers once the server listens. The image's
  `HEALTHCHECK` asks it, and mundos reads the container's health to say a
  generation is running.

## Updating a world

- Never in place. mundos makes the world's next generation on the chosen
  version and fills its folder before it first starts, by running the
  source's own image: `planet copy /from /to` copies the folder, each SQLite
  file through its online backup while the source keeps running (p). Then
  mundos copies the bucket folder, on the bucket's side.
- The new version moves each plugin's store forward when it starts, on the
  copy. The old generation is untouched and is the way back.
- Never onto an older version: a store moved forward is not promised to open
  on an older one.
- Someone walks the copy at its own address, and a promotion gives it the
  world's.

## Development

- `bun run dev` runs a world alone: the Go server and Vite on one origin, no
  mundos, every session an `admin` through `PLANET_DEV_LEVEL`.
- Against a local mundos: build the image here and mundos's worlds page lists
  it among the versions this machine holds.

## What to measure once it is up

- Round trip from a browser in Brazil to the socket: what sets how far behind
  a peer is drawn.
- The actor's frame size and rate in the server log, at a handful of players.
- Time to first frame on a cold cache, dominated by the field download.

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
  pulls anonymously, as it does the Hyperfy fork.
- In it: the Go binary, the web front end's files, the engine's WASM, and the
  default asset set. One process runs (90), as a user of its own. The
  `Dockerfile` at the root builds it, with BuildKit.
- The image sets its own variables: `HOST=0.0.0.0`, `PORT=3000`,
  `WORLD_DIR=/world` and `WEB_DIR`, where the page's files are.
- The Go binary is static, and the engine and the page are the same on every
  architecture: the three builds run on the building machine and only the
  last stage differs.
- What publishes a version, and whether the image carries the baked fields or
  finds them outside it, waits on where fields are hosted (OPEN.md). Built on
  a machine that baked them, the image has them.

## What mundos sets

| Variable | What |
|---|---|
| `PORT` | where the server listens inside the container: 3000, as the image says |
| `MUNDOS_PUBLIC_KEY` | the Ed25519 public key that checks tokens: PEM, its newlines escaped |
| `PUBLIC_MUNDOS_URL` | mundos's origin, where the door is |
| `PUBLIC_MUNDOS_WORLD` | the world's name, the token's audience |

- The names are the ones the Hyperfy fork reads, so mundos says the same thing
  to every instance type. The bucket's two, `ASSETS_S3_URI` and
  `ASSETS_BASE_URL`, join them when a world has files to keep (89).
- The port is published on `127.0.0.1` alone. mundos's Caddy is the one thing
  that reaches it, with the world's address and its certificate, and it
  carries the socket with no extra setting.

## The world folder

- A Docker volume, named by mundos after the generation's id and mounted at
  `/world`, which the image gives to the user the server runs as. In it:
  `world.json`, the recipe, and each plugin's SQLite file (89).
- It is the whole world: deleting a generation deletes its folder and its
  bucket folder.

## Health

- `GET /api/health` answers once the server listens. The image's
  `HEALTHCHECK` asks it, and mundos reads the container's health to say a
  generation is running.

## Updating a world

- By copy. mundos makes the world's next generation on the chosen version
  and fills its folder before it first starts, by running the
  source's own image: `planet copy /from /world`, the source read-only at
  `/from` and the new volume where a world's own sits. Every file goes as it
  is while the source keeps running; a plugin's SQLite file goes through its
  online backup, when the first one exists. Then mundos copies the bucket
  folder, on the bucket's side.
- The new version moves each plugin's store forward when it starts, on the
  copy. The old generation stays as it is, and is the way back.
- Onto the same version or a newer one: a store moves forward only.
- Someone walks the copy at its own address, and a promotion gives it the
  world's.

## Development

- `bun run dev` runs a world alone: the Go server and Vite on one origin,
  every session an `admin` through `PLANET_DEV_LEVEL`.
- Against a local mundos: build the image here and mundos's worlds page lists
  it among the versions this machine holds.

## What to measure once it is up

- Round trip from a browser in Brazil to the socket: what sets how far behind
  a peer is drawn.
- The actor's frame size and rate in the server log, at a handful of players.
- Time to first frame on a cold cache, dominated by the field download.

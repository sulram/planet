# DEPLOY

How an instance is hosted: the box, the region, the front door, the bucket,
and what is on before the door opens. The planes and the seams are in
ARCHITECTURE.md; the open rows this waits on are in OPEN.md.

## What the server carries

- Rendering, terrain generation and collision run in the browser. The server
  never generates a patch.
- PocketBase is SQLite in one process; the world actor relays small protobuf
  frames. Twenty people in one world is 20 x 19 x 15 frames a second, under
  1 MB/s.
- The heavy bytes are static and cached forever: the WASM engine (a few MB),
  each avatar VRM, and a field (25 MB) on a first visit.

## The box

| | |
|---|---|
| CPU | 2 vCPU. ARM is fine: the Go binary has no CGO, and the Pi is a target already |
| RAM | 4 GB. The process sits under 200 MB today; the headroom is for world actors, the wazero generator and SQLite's page cache when chunks arrive (ROADMAP M3) |
| Disk | 40 to 80 GB NVMe, local. SQLite in WAL mode wants local SSD, never network storage |
| Traffic | 20 TB is the standard at this class and is far more than presence needs |

- Grow it only when a measurement says so.

## The region

- The players' round trip is what presence feels; where the operator sits
  matters nothing. The client draws a peer about 100 ms behind its newest
  stance, and the round trip adds to that.
- Players are mainly in Brazil: the world server goes to Sao Paulo. From
  there Hetzner's nearest locations are Ashburn (120 to 140 ms) and
  Falkenstein (over 200 ms): walking together is fine at either, anything you
  chase wants the local box.
- Sao Paulo at this class: Hostinger KVM 2 (2 vCPU, 8 GB, 100 GB NVMe) for
  about R$ 45 to 80 a month on a prepaid term, which is where the first
  instance runs; Vultr and Akamai (Linode) at US$18 to 24 with hourly billing;
  Magalu Cloud bills in BRL. The hyperscalers have the region and cost more
  than the whole bill for nothing this stack uses.
- Hetzner keeps what does not care about latency: the asset bucket, backups
  of `pb_data` and the `world.db` files, and a staging instance.
- Decide with numbers: `mtr` from a Brazilian connection to a candidate IP,
  then a second tab walked beside the first, from Brazil, once it is up.

## The front door: Caddy

- Caddy is a web server in Go, used as a reverse proxy. It gets and renews
  TLS certificates from Let's Encrypt by itself and proxies WebSockets with
  no extra setting: one binary, a few lines, in the spirit of the stack.
- One origin on 443. `PB_PUBLIC_URL` is then the site's own origin, cookies
  are same site, and there is no cross origin socket to reason about. The
  socket on 443 is also what survives a rough network between an operator
  abroad and the box.

```text
planet.example.org {
    handle /api/* {
        reverse_proxy 127.0.0.1:8090
    }
    handle /_/* {
        reverse_proxy 127.0.0.1:8090
    }
    handle {
        reverse_proxy 127.0.0.1:3000
    }
}
```

- `/api` is the REST, the ticket and the world socket; `/_` is the PocketBase
  panel; everything else is the SvelteKit node server (adapter-node, port
  3000).
- While another service holds 443 on the box (the first instance shares it
  with a VPN), the site lives on `DEPLOY_<ENV>_HTTPS_PORT`: Caddy sets
  `https_port`, gets the certificate by the HTTP challenge on 80, and the
  origin carries the port. Moving to 443 is changing that one variable and
  provisioning again.

## Provision and deploy

- Two scripts, both from this machine over the ssh key, both with
  `--dry-run` that prints every command. Where they go is `DEPLOY_<ENV>_*`
  in the local `.env` (`.env.example`); nothing per box is in git.
- `bun run provision <env>`, once per box, as root, idempotent: the `planet`
  system user and `/opt/planet`; Bun for it, the version this machine runs;
  Caddy from its apt repository and the Caddyfile above; ufw rules for 80 and
  the https port; the units `planet-server` and `planet-web`. It refuses to
  run until `/etc/planet/<env>.env` exists.
- `/etc/planet/<env>.env` is written by hand once, root only: the variables
  of `.env.example` with the public origin, plus `ORIGIN`, `HOST=127.0.0.1`
  and `PORT=3000` for adapter-node. Both units read it. A change there is
  `systemctl restart planet-server planet-web`.
- `bun run deploy <env>`, every release: builds here (the Go binary for the
  box's arch with no CGO, the WASM engine, the web app), never on the box;
  uploads a release to `/opt/planet/releases/<stamp>` and the asset set to
  `/opt/planet/assets`, shared by every release and linked into each, so a
  deploy moves only what changed of it; swaps `/opt/planet/current`, restarts
  the server and checks `/api/health`, upserts the superuser from the env,
  restarts the web app and checks it answers. A failed check swaps the
  previous release back. The last five releases stay for a rollback by hand.
- A release holds the binary, the web build and the one package its server
  side imports at run time (`pocketbase`, from `apps/web/package.json`
  dependencies). `pb_data` lives outside the releases and survives them.
- Logs: `journalctl -u planet-server -f`, `journalctl -u planet-web -f`,
  `journalctl -u caddy -f`. With `SMTP_HOST` empty the magic links are in the
  first one.

## The bucket: Hetzner Object Storage

- PocketBase's file storage has an S3 mode and Hetzner Object Storage speaks
  S3: endpoint `https://<location>.your-objectstorage.com`, bucket, region
  (the location, `fsn1`), access key and secret, force path style on. Every
  file field of every collection then lives in the bucket instead of
  `pb_data/storage`.
- Those settings belong in `server/internal/cold/settings.go` as `S3_*`
  variables beside `SMTP_*`, applied from the environment on every start,
  never set by hand in the panel. Not wired yet.
- The same bucket serves what ARCHITECTURE already draws: PocketBase keeps
  records and small files; heavy media goes straight to the bucket by
  presigned URL and is read from the bucket's URL. The WASM engine, the
  default avatars and the fields can live there too, a CDN in front of it
  later being a flag and not a rewrite.

## Before the door opens

- The two rows in OPEN: PocketBase rate limits are off and the code request
  creates accounts, so a bot could mint users; SMTP needs real credentials or
  nobody signs in.
- `APP_URL` is the public origin (magic links point there), `PB_URL` the
  binary's local address, `PB_PUBLIC_URL` the public origin again.
- `PLANET_OPERATOR_EMAIL` names the first operator; the superuser is the
  panel login, never a person in a world.

## What to measure once it is up

- Round trip from a browser in Brazil to the socket: what sets how far behind
  a peer is drawn.
- The actor's frame size and rate in the server log, at a handful of players.
- Time to first frame on a cold cache, dominated by the field download.

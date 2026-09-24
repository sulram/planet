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
- Sao Paulo at this class: Vultr and Akamai (Linode), 2 vCPU and 4 GB for
  about US$18 to 24 a month; Brazilian clouds (Magalu Cloud) bill in BRL. The
  hyperscalers have the region and cost more than the whole bill for nothing
  this stack uses.
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

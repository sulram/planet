# 72. A field is fetched once: its URL carries the content id and its bytes travel as brotli (decided)

Logged 2026-09-28.

The URL a page hands the engine for a field is the file's path with the
content id as its query, and the front door marks `/assets/fields/*.field`
immutable for a year. `bun run deploy` writes a brotli sibling beside every
field, once per bake, and the static server sends it to a browser that
accepts brotli.

Entering the Earth world took a long wait on every visit. The field went out
as 25 MB with no content type, so Caddy did not compress it, and with no
cache header, so a browser fetched it again whenever it had let the bytes go.
DEPLOY.md said the heavy bytes were cached forever; the WASM was, since Vite
hashes its name, and the field was not, since it is served by name from the
asset set.

The bytes are named by their id already: the sidecar carries it and the
recipe pins it. Putting the id in the URL makes the URL as immutable as the
content, so a rebake is a new URL and an old one can be kept for good. The
static server, sirv under adapter-node, already serves a `.br` sibling when
there is one; making it at deploy time, where every other artefact is built,
costs 38 seconds once per bake and turns 25.3 MB into 15.8. Caddy compressing
on the fly does not apply: it matches on content type, the file has none, and
it would spend the box's CPU on every cold fetch for a worse ratio.

**Rejected:** renaming the files to their ids, which moves the bake, the
sidecar lookup and the explorer for what a query string does; serving fields
through a SvelteKit route to set the headers in the app, which streams 25 MB
through the node server on every cold fetch and puts a static file behind a
dynamic one; a cache header at the proxy without the id in the URL, which
would keep a stale Earth after a rebake; brotli at bake time, which ties the
data to how one server ships it.

**Lives in:** DEPLOY.md, `apps/web/src/lib/world.ts` (`fieldUrl`),
`scripts/deploy.ts`, `scripts/provision.ts`.

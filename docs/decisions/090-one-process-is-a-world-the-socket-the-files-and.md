# 90. One process is a world: the socket, the files and a front end that is only files (decided)

Logged 2026-10-04.

The Go binary is the whole instance. It serves the world socket, the world's
routes, the heavy files when there is no bucket, and the web front end as
static files, built by SvelteKit's static adapter. Nothing of Node or Bun runs
in a world.

**Removes:** 23.

**Why the page needs no server of its own.** 23 put SvelteKit on a node server
for the session cookie, the form actions and the guarded routes. 87 took all
three: there is no session, no form, no backoffice. What the page reads, the
recipe and what the world says it is, comes from the world's routes; what it
keeps, the language, the theme, the avatar and a visitor's name, belongs to
the browser. 23 refused a static app because a session token would sit where
scripts read it. There is no such token now: mundos's lives a minute, arrives
in the fragment and is spent on the socket.

**Why one process.** mundos routes an address to one port of one container.
Two processes behind it need a proxy between them and something to keep both
alive, and the image carries a JavaScript runtime to render a page that is a
canvas. One origin also ends `PB_PUBLIC_URL` and every thought about a socket
across origins.

**Why it matters past the web.** The web front end is one front end among
those to come. Logic in a server half of its own is logic a desktop or a
headset cannot reach, which is what kept the desktop a viewer with flags.
With the page only files, what a front end may do is exactly the seam and the
world's routes.

**What moves.** The field's brotli sibling (72) and its year of cache are sent
by the Go server. In development Vite serves the page and proxies the world's
routes and socket to the Go server, so one origin holds there too.

**Rejected:** SvelteKit on a node server beside Go in one container; pages
rendered by Go templates, with the panels still Svelte (17) and written twice;
a JavaScript server in front with Go behind it, the same two processes the
other way round.

**Lives in:** ARCHITECTURE.md § The server and § Clients and UI; DEPLOY.md;
BRIEF.md.

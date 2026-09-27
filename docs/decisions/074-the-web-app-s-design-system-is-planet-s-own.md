# 74. The web app's design system is planet's own, grown for the metaverse (decided)

Logged 2026-09-28.

The web app keeps `$lib/ds`: JetBrains Mono at one size, black and white
with one red, radius 0, light and dark. It grows with what the world asks
of it, a panel over the picture, a readout, a code read from the mail, and
answers to no other product.

The studio's design system, `@tekne/ds`, was worn for a day: installed
from the studio's registry, in its mono theme, with a planet theme over it
for glass panels. It did not fit the world. A system for the metaverse
alone lets planet's pieces be shaped by the world and nothing else, and
keeps a clone installable without a private registry and its token.

**Rejected:** `@tekne/ds` with a planet theme on top; giving planet's
components to it, which puts metaverse pieces in a system whose other
consumers do not need them.

**Lives in:** ARCHITECTURE.md § Clients and UI, `apps/web/src/lib/ds`.

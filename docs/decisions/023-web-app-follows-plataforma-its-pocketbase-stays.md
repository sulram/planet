# 23. Web app follows Plataforma ITS; PocketBase stays a Go library (decided)

Logged 2026-09-20.

SvelteKit on adapter-node, a PocketBase client per request, the session in an
httpOnly cookie, OTP as magic link, the design system in `$lib/ds` with a live
catalogue, flat i18n keys, scripts in Bun: all as in Plataforma ITS, which
Marlus runs in production. The one difference is decision 10: PocketBase is
embedded in our Go binary, not run as its own binary, because the hot plane
needs its permission cache invalidated by hooks in the same process. Accounts
are created by a server hook on the first code request, so the web app holds
no superuser credentials. Rejected: a static SPA served by Go (the token would
live in storage readable by scripts).

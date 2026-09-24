# 27. PocketBase 0.40 needs Go 1.27 (decided)

Logged 2026-09-20.

The 0.40 line declares `go 1.27`. We pin 0.40.4 exactly and let the Go
toolchain fetch itself. The recipe freeze is a validate hook, not an API rule,
because rules skip superusers and Go side saves.

# 21. Versions count from 0.0.1; `dev` publishes prereleases (decided)

Logged 2026-09-20.

Marlus asked for a `dev` branch that counts `0.0.1-dev.N` before anything
reaches `main`. Semantic Release does it with a prerelease branch and the tag
`v0.0.0` as anchor. Before 1.0.0 the rules are: BREAKING CHANGE bumps minor,
feat, fix and perf bump patch, so 1.0.0 is a deliberate act. At 1.0.0 the
custom rules are deleted and the defaults apply. Rejected: the default rules
from day one (the first feat would publish 1.0.0).

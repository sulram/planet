# 24. Operator, a global flag (decided)

Logged 2026-09-20.

The backoffice needs a gate, and roles are per world. An operator is a user
with `users.operator = true`: the person who runs the instance. It is not the
PocketBase superuser (panel login) and not the per world admin. Guarded by an
API rule with `:changed`, so only operators change it, and user creation over
the API is closed because the default public create would allow self grant.

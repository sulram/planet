---
group: hosting
owner: marlus
unblocks: the bucket mode of the server (89), the planet adapter in mundos passing ASSETS_S3_URI and ASSETS_BASE_URL, and a migration script
since: 2026-10-20
context:
  - DEPLOY.md
  - ARCHITECTURE.md#files
---

# When a world's files move to the bucket

Today every file of planeta.museu.xyz lives in the world folder on the Hostinger box, served by the Go server at /files, and that performs well. The move to the bucket (89) waits until a world weighs: then the server writes to S3 (Backblaze, the same bucket and CDN as the Hyperfy fork on mundos), the planet adapter in mundos passes the two variables, and a script, likely in mundos, migrates the folder's files into the generation's prefix. Until then, no bucket code.

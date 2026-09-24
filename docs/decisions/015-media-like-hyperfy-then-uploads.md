# 15. Media like Hyperfy, then uploads (decided; details proposed)

Logged 2026-09-20.

Public URLs on a CDN are allowed. Because building is restricted to trusted
people, uploads (image, video, GLB) are allowed too, stored on the Hetzner
Object Storage already attached to PocketBase. Later business: sell data
packages, which is only a quota number. Proposed details: thumbnails made by
the owner's client, heavy media served straight from the bucket, proximity
loading with a decoder budget. Known cost of external URLs: CORS and visitor
IP exposure.

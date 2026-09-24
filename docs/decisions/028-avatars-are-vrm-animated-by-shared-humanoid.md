# 28. Avatars are VRM, animated by shared humanoid clips (decided)

Logged 2026-09-20.

Marlus supplied a first set of CC0 VRM avatars and asked for Hyperfy's basic
locomotion: idle, walk, jog, jump, fall, float. Clips are authored once on a
Mixamo rig and retargeted at load to the VRM humanoid, so every avatar shares
them. An anonymous visitor gets a random avatar from the default set, kept in a
cookie. Changing avatar happens in the world, not in a menu: a changer entity
you walk through opens a dialog, and a dropzone lets a builder place a VRM on
the map (both on the ROADMAP). The box figure stays as the fallback while
assets load. This settles the OPEN avatar format question for VRM; custom voxel
avatars stay a wish. Hyperfy is read for architecture only. Its clip files (GPL) are
committed as a temporary default set by Marlus's call, to be replaced (see OPEN).

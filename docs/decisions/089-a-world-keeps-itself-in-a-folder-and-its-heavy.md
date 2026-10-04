# 89. A world keeps itself in a folder, and its heavy files in a bucket (decided)

Logged 2026-10-04.

The server keeps a world in its own folder: the recipe and what its admin
set, and one SQLite file for each plugin that keeps things. Heavy files, an
image, a video, a GLB, are named by the hash of their content and live in the
bucket folder mundos hands the generation, as the Hyperfy fork keeps its own.
The server stands on the standard library, the socket and protobuf, with
SQLite direct when the first plugin keeps things.

**Removes:** 10, 27.

**Why a folder is enough.** 10 gave PocketBase accounts, worlds, volumes,
roles and records, and the panel to edit them. Accounts and roles are mundos's, an
instance is one world, and nobody may sign in to a panel in a world that
stores no password. What is left is one row, the recipe. A framework pinned
before its 1.0, with a Go toolchain of its own (27), is a lot to carry for a
row.

**How files travel.** A session that may build sends the file to the world
server, which takes it only from a live session of that level, names it
`<hash>.<ext>` and writes it to the folder mundos named for this generation.
Browsers read it from the address in front of the bucket; a record says
`asset://<hash>.<ext>`, so moving the bucket is one variable. With no bucket
set, the files stay in the world folder and the server serves them: a world on
a laptop needs no account anywhere. On start a world deletes from its bucket
folder what no record names.

**Why a file for each plugin.** A plugin switched off leaves its file whole
and unread, its schema moves forward by its own migrations, and a plugin
dropped from a version takes one file with it. The chunks and the op log of 10
are the build plugin's.

**What an upgrade copies.** The folder, each SQLite file through its online
backup while the source keeps running, then the bucket folder on the bucket's
side. mundos does both for Hyperfy today.

**Rejected:** PocketBase kept for one row and a panel nobody signs in to; one
`world.db` for every plugin, which ties their schemas and their fates; files
named by a person, which collide and cannot be kept by a browser for good;
uploads signed straight to the bucket, which skip the one place that knows who
is uploading, and wait for a video heavy enough to ask; files shared between
worlds, since a folder of its own is what makes an upgrade a copy and a
deletion a folder.

**Lives in:** ARCHITECTURE.md § The server and § Files; DEPLOY.md; BRIEF.md.

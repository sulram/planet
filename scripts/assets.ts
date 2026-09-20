// Imports the default asset set into `assets/` from the reference checkouts in
// `refs/`: the avatars and the locomotion clips. The result is committed; this
// script records where it came from. See docs/OPEN.md, Default asset set.
import { $ } from 'bun';
import { readdir } from 'node:fs/promises';
import { ROOT } from './lib';

const OUT = `${ROOT}/assets`;
const AVATARS = `${ROOT}/refs/avatars`;
const CLIPS = `${ROOT}/refs/hyperfy/src/world/assets`;

// Clip name in the engine <- file in the reference checkout.
const CLIP_FILES = {
	idle: 'mp-idle.glb',
	walk: 'mp-walk.glb',
	run: 'mp-jog.glb',
	jump: 'emote-jump.glb',
	fall: 'emote-fall.glb',
	fly: 'emote-float.glb'
};

await $`mkdir -p ${OUT}/avatars ${OUT}/clips`;
const avatars = (await readdir(AVATARS)).filter((f) => f.endsWith('.vrm')).sort();
for (const file of avatars) await $`cp ${AVATARS}/${file} ${OUT}/avatars/${file}`;
for (const [name, file] of Object.entries(CLIP_FILES)) await $`cp ${CLIPS}/${file} ${OUT}/clips/${name}.glb`;

// `assets/manifest.json` is the config: which avatars are offered and which
// clip plays for each gait. It is edited by hand, never written here.
console.log(`${avatars.length} avatars, ${Object.keys(CLIP_FILES).length} clips -> ${OUT}`);

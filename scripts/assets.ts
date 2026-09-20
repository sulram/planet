// Imports the default asset set into `assets/` from the reference checkouts in
// `refs/`: the avatars and the locomotion clips. The result is committed; this
// script records where it came from. See docs/OPEN.md, Default asset set.
import { $ } from 'bun';
import { readdir } from 'node:fs/promises';
import { ROOT } from './lib';

const OUT = `${ROOT}/assets`;
const AVATARS = `${ROOT}/refs/avatars`;
const HYPERFY = `${ROOT}/refs/hyperfy/src/world/assets`;

// Clip name in the engine <- source file.
const CLIP_FILES = {
	idle: `${HYPERFY}/mp-idle.glb`,
	walk: `${HYPERFY}/mp-walk.glb`,
	run: `${HYPERFY}/mp-jog.glb`,
	jump: `${HYPERFY}/emote-jump.glb`,
	fall: `${HYPERFY}/emote-fall.glb`,
	fly: `${HYPERFY}/emote-float.glb`,
	swim: `${ROOT}/refs/swim.glb`
};

/**
 * A GLB reduced to what a clip needs: nodes and animations. Meshes, skins,
 * materials and images go, and the binary chunk keeps only the keyframes.
 * Downloads from Mixamo carry the whole character; the engine never reads it.
 */
function clipOnly(glb: Uint8Array): Uint8Array {
	const view = new DataView(glb.buffer, glb.byteOffset, glb.byteLength);
	const jsonLength = view.getUint32(12, true);
	const json = JSON.parse(new TextDecoder().decode(glb.subarray(20, 20 + jsonLength)));
	const bin = glb.subarray(20 + jsonLength + 8);

	// Accessors the animations use, renumbered, each with its own buffer view.
	const kept = new Map<number, number>();
	const chunks: Uint8Array[] = [];
	const accessors: unknown[] = [];
	const bufferViews: unknown[] = [];
	let offset = 0;
	const keep = (index: number): number => {
		if (kept.has(index)) return kept.get(index)!;
		const accessor = json.accessors[index];
		const source = json.bufferViews[accessor.bufferView];
		if (source.byteStride) throw new Error('interleaved keyframes are not supported');
		const components = { SCALAR: 1, VEC3: 3, VEC4: 4 }[accessor.type as 'SCALAR' | 'VEC3' | 'VEC4'];
		const bytes = accessor.count * components * 4; // keyframes are f32
		const start = (source.byteOffset ?? 0) + (accessor.byteOffset ?? 0);
		chunks.push(bin.subarray(start, start + bytes));
		bufferViews.push({ buffer: 0, byteOffset: offset, byteLength: bytes });
		accessors.push({ ...accessor, bufferView: bufferViews.length - 1, byteOffset: 0 });
		offset += bytes; // multiples of 4 already
		kept.set(index, accessors.length - 1);
		return accessors.length - 1;
	};
	const animations = json.animations.map((animation: any) => ({
		...animation,
		samplers: animation.samplers.map((sampler: any) => ({
			...sampler,
			input: keep(sampler.input),
			output: keep(sampler.output)
		}))
	}));
	const nodes = json.nodes.map(({ mesh, skin, ...node }: any) => node);
	const out = {
		asset: json.asset,
		scene: json.scene,
		scenes: json.scenes,
		nodes,
		animations,
		accessors,
		bufferViews,
		buffers: [{ byteLength: offset }]
	};

	const text = new TextEncoder().encode(JSON.stringify(out));
	const jsonChunk = new Uint8Array(Math.ceil(text.length / 4) * 4).fill(0x20);
	jsonChunk.set(text);
	const total = 12 + 8 + jsonChunk.length + 8 + offset;
	const file = new Uint8Array(total);
	const header = new DataView(file.buffer);
	header.setUint32(0, 0x46546c67, true); // glTF
	header.setUint32(4, 2, true);
	header.setUint32(8, total, true);
	header.setUint32(12, jsonChunk.length, true);
	header.setUint32(16, 0x4e4f534a, true); // JSON
	file.set(jsonChunk, 20);
	header.setUint32(20 + jsonChunk.length, offset, true);
	header.setUint32(24 + jsonChunk.length, 0x004e4942, true); // BIN
	let at = 28 + jsonChunk.length;
	for (const chunk of chunks) {
		file.set(chunk, at);
		at += chunk.length;
	}
	return file;
}

await $`mkdir -p ${OUT}/avatars ${OUT}/clips`;
const avatars = (await readdir(AVATARS)).filter((f) => f.endsWith('.vrm')).sort();
for (const file of avatars) await $`cp ${AVATARS}/${file} ${OUT}/avatars/${file}`;
for (const [name, path] of Object.entries(CLIP_FILES)) {
	const source = new Uint8Array(await Bun.file(path).arrayBuffer());
	await Bun.write(`${OUT}/clips/${name}.glb`, clipOnly(source));
}

// `assets/manifest.json` is the config: which avatars are offered and which
// clip plays for each gait. It is edited by hand, never written here.
console.log(`${avatars.length} avatars, ${Object.keys(CLIP_FILES).length} clips -> ${OUT}`);

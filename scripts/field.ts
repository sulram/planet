// Bakes a field: the ground of a real body, as a cube map our sectors read.
//   bun run field
// The source is ETOPO 2022 (NOAA NCEI), topography and bathymetry in one
// grid, a work of the United States government and so in the public domain.
// It is fetched once into `target/field/`, and only the bake is an asset.
//
// Our planet is 1/305 of Earth, so the elevations are not heights here: what
// survives the change of scale is the shape. See `crates/worldgen/src/v3.rs`.
import { $ } from 'bun';
import { mkdir, readdir } from 'node:fs/promises';
import { ROOT } from './lib';

// --- The source grid -------------------------------------------------------

/** ETOPO 2022 at 60 arc-seconds, as the OPeNDAP server names it. */
const DODS =
	'https://www.ngdc.noaa.gov/thredds/dodsC/global/ETOPO2022/60s/60s_surface_elev_netcdf/ETOPO_2022_v1_60s_N90W180_surface.nc.dods';
const SRC_ROWS = 10800;
const SRC_COLS = 21600;
/** Every other sample: 3.7 km on Earth, still far finer than a texel here. */
const STRIDE = 2;
/** Source rows per request. Whole grid in one is refused halfway through. */
const BAND = 900;

const ROWS = SRC_ROWS / STRIDE;
const COLS = SRC_COLS / STRIDE;

// --- The cube map ----------------------------------------------------------

/** Texels per face side. 32 m of planet per texel, 9.8 km of Earth. */
const SIDE = 1024;
/** Levels down to a 16 texel face: enough for the coarsest patch in orbit. */
const LEVELS = 7;
/** Metres of ruggedness in one step of the byte. Matches `field.rs`. */
const RUGGED_STEP = 16;
const SECTOR_SIDE = 65536;
const HALF = SECTOR_SIDE / 2;
const QUARTER_PI = Math.PI / 4;

/** `[n, u, v]` per sector, in the order `crates/topology` fixes. */
const FRAMES: number[][][] = [
	[[1, 0, 0], [0, 1, 0], [0, 0, 1]],
	[[-1, 0, 0], [0, 0, 1], [0, 1, 0]],
	[[0, 1, 0], [0, 0, 1], [1, 0, 0]],
	[[0, -1, 0], [1, 0, 0], [0, 0, 1]],
	[[0, 0, 1], [1, 0, 0], [0, 1, 0]],
	[[0, 0, -1], [0, 1, 0], [1, 0, 0]]
];

const dot = (a: number[], b: number[]) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];

/** Block coordinate `0..=SECTOR_SIDE` to cube face coordinate, warped. */
const warp = (blocks: number) => Math.tan(((blocks - HALF) / HALF) * QUARTER_PI);
const unwarp = (face: number) => (Math.atan(face) / QUARTER_PI) * HALF + HALF;

/** The unit vector through a surface point. Legal outside the face: that is
 *  exactly what fills a gutter with the ground that continues past the edge. */
function direction(sector: number, u: number, v: number): number[] {
	const [n, ua, va] = FRAMES[sector];
	const [x, y] = [warp(u), warp(v)];
	const q = [0, 1, 2].map((a) => n[a] + ua[a] * x + va[a] * y);
	const len = Math.hypot(q[0], q[1], q[2]);
	return [q[0] / len, q[1] / len, q[2] / len];
}

/** The sector and surface coordinates a direction passes through. */
function fromDirection(d: number[]): [number, number, number] {
	let axis = 0;
	for (let a = 1; a < 3; a++) if (Math.abs(d[a]) > Math.abs(d[axis])) axis = a;
	const sector = axis * 2 + (d[axis] < 0 ? 1 : 0);
	const [n, ua, va] = FRAMES[sector];
	const depth = dot(d, n);
	return [sector, unwarp(dot(d, ua) / depth), unwarp(dot(d, va) / depth)];
}

// --- Fetching --------------------------------------------------------------

const CACHE = `${ROOT}/target/field`;

/** One latitude band of the source, as metres of elevation. */
async function band(index: number): Promise<Float32Array> {
	const file = `${CACHE}/band-${String(index).padStart(2, '0')}.dods`;
	const cached = Bun.file(file);
	if (!(await cached.exists())) {
		const first = index * BAND;
		const query = `?z.z[${first}:${STRIDE}:${first + BAND - 1}][0:${STRIDE}:${SRC_COLS - 1}]`;
		process.stdout.write(`  band ${index + 1}/${SRC_ROWS / BAND}\r`);
		const res = await fetch(DODS + encodeURI(query));
		if (!res.ok) throw new Error(`ETOPO band ${index}: HTTP ${res.status}`);
		await Bun.write(file, await res.arrayBuffer());
	}
	const bytes = new Uint8Array(await Bun.file(file).arrayBuffer());
	const mark = new TextEncoder().encode('Data:\n');
	let at = 0;
	outer: for (; at < bytes.length - mark.length; at++) {
		for (let k = 0; k < mark.length; k++) if (bytes[at + k] !== mark[k]) continue outer;
		break;
	}
	// `Data:` then the array length, twice, then big endian floats.
	const start = at + mark.length + 8;
	const wanted = (BAND / STRIDE) * COLS;
	if (bytes.length - start < wanted * 4) throw new Error(`ETOPO band ${index} is short; delete ${file}`);
	const view = new DataView(bytes.buffer, bytes.byteOffset + start);
	const out = new Float32Array(wanted);
	for (let i = 0; i < wanted; i++) out[i] = view.getFloat32(i * 4, false);
	return out;
}

// --- Baking ----------------------------------------------------------------

type Face = { elevation: Float64Array; rugged: Float64Array };

/** One pass over the source: every sample lands in the texel that holds it,
 *  which is the only way to cover a sphere whose texels are not all equal. */
async function scatter(): Promise<Face[]> {
	const texels = SIDE * SIDE;
	const sum = Array.from({ length: 6 }, () => new Float64Array(texels));
	const count = Array.from({ length: 6 }, () => new Uint32Array(texels));
	const low = Array.from({ length: 6 }, () => new Float32Array(texels).fill(Infinity));
	const high = Array.from({ length: 6 }, () => new Float32Array(texels).fill(-Infinity));

	// Longitude is the same for every row: pay for it once.
	const cosLon = new Float64Array(COLS);
	const sinLon = new Float64Array(COLS);
	for (let c = 0; c < COLS; c++) {
		const lon = ((-180 + (c * STRIDE + 0.5) / 60) * Math.PI) / 180;
		cosLon[c] = Math.cos(lon);
		sinLon[c] = Math.sin(lon);
	}

	for (let b = 0; b < SRC_ROWS / BAND; b++) {
		const rows = await band(b);
		const height = BAND / STRIDE;
		for (let r = 0; r < height; r++) {
			// ETOPO counts rows from the south pole up, whatever its name says.
			const lat = ((-90 + (b * BAND + r * STRIDE + 0.5) / 60) * Math.PI) / 180;
			// Earth's pole is our pole: `material` reads latitude off `d[1]`.
			const [y, ring] = [Math.sin(lat), Math.cos(lat)];
			for (let c = 0; c < COLS; c++) {
				const elevation = rows[r * COLS + c];
				const [sector, u, v] = fromDirection([ring * cosLon[c], y, ring * sinLon[c]]);
				const tx = Math.min(SIDE - 1, Math.max(0, Math.floor((u / SECTOR_SIDE) * SIDE)));
				const ty = Math.min(SIDE - 1, Math.max(0, Math.floor((v / SECTOR_SIDE) * SIDE)));
				const at = ty * SIDE + tx;
				sum[sector][at] += elevation;
				count[sector][at]++;
				if (elevation < low[sector][at]) low[sector][at] = elevation;
				if (elevation > high[sector][at]) high[sector][at] = elevation;
			}
		}
		process.stdout.write(`  scattered ${(((b + 1) / (SRC_ROWS / BAND)) * 100).toFixed(0)}%\r`);
	}

	return sum.map((_, sector) => {
		const elevation = new Float64Array(texels);
		const rugged = new Float64Array(texels);
		for (let at = 0; at < texels; at++) {
			// A corner texel of a warped face is the smallest there is; none
			// come out under a source sample, but say so if one ever does.
			if (count[sector][at] === 0) throw new Error(`texel ${at} of face ${sector} saw no source`);
			elevation[at] = sum[sector][at] / count[sector][at];
			rugged[at] = high[sector][at] - low[sector][at];
		}
		return { elevation, rugged };
	});
}

/** Half the texels, the same ground: both channels are means, so ruggedness
 *  stays the measure of one finest texel and a silhouette cannot change
 *  because the camera moved away. */
function halve(face: Face, side: number): Face {
	const half = side / 2;
	const out: Face = { elevation: new Float64Array(half * half), rugged: new Float64Array(half * half) };
	for (let y = 0; y < half; y++) {
		for (let x = 0; x < half; x++) {
			let e = 0;
			let r = 0;
			for (let oy = 0; oy < 2; oy++) {
				for (let ox = 0; ox < 2; ox++) {
					const at = (y * 2 + oy) * side + x * 2 + ox;
					e += face.elevation[at];
					r += face.rugged[at];
				}
			}
			out.elevation[y * half + x] = e / 4;
			out.rugged[y * half + x] = r / 4;
		}
	}
	return out;
}

/** Writes one level: six faces, each with a one texel gutter carrying the
 *  ground from across its seam, so nothing at runtime knows what a seam is. */
function writeLevel(into: Uint8Array, offset: number, faces: Face[], side: number): number {
	const stride = side + 2;
	const plane = stride * stride;
	// Bilinear read of a face at surface coordinates, for the gutter.
	const read = (face: Face, u: number, v: number, channel: 'elevation' | 'rugged') => {
		const x = Math.min(side - 0.5001, Math.max(0.5, (u / SECTOR_SIDE) * side)) - 0.5;
		const y = Math.min(side - 0.5001, Math.max(0.5, (v / SECTOR_SIDE) * side)) - 0.5;
		const [x0, y0] = [Math.floor(x), Math.floor(y)];
		const [fx, fy] = [x - x0, y - y0];
		const at = (ox: number, oy: number) => face[channel][(y0 + oy) * side + x0 + ox];
		const top = at(0, 0) + (at(1, 0) - at(0, 0)) * fx;
		const bottom = at(0, 1) + (at(1, 1) - at(0, 1)) * fx;
		return top + (bottom - top) * fy;
	};

	for (let sector = 0; sector < 6; sector++) {
		const face = faces[sector];
		const elevation = new Int16Array(plane);
		const rugged = new Uint8Array(plane);
		for (let b = 0; b < stride; b++) {
			for (let a = 0; a < stride; a++) {
				let e: number;
				let r: number;
				if (a > 0 && b > 0 && a <= side && b <= side) {
					const at = (b - 1) * side + (a - 1);
					e = face.elevation[at];
					r = face.rugged[at];
				} else {
					// Outside the face: the direction still resolves, onto
					// whichever sector actually holds that ground.
					const u = ((a - 0.5) / side) * SECTOR_SIDE;
					const v = ((b - 0.5) / side) * SECTOR_SIDE;
					const [other, ou, ov] = fromDirection(direction(sector, u, v));
					e = read(faces[other], ou, ov, 'elevation');
					r = read(faces[other], ou, ov, 'rugged');
				}
				elevation[b * stride + a] = Math.max(-32768, Math.min(32767, Math.round(e)));
				rugged[b * stride + a] = Math.max(0, Math.min(255, Math.round(r / RUGGED_STEP)));
			}
		}
		into.set(new Uint8Array(elevation.buffer), offset);
		offset += plane * 2;
		into.set(rugged, offset);
		offset += plane;
	}
	return offset;
}

/** An equirectangular look at what was baked, so the bake can be seen and not
 *  only believed (CLAUDE.md, How it grows). */
function preview(faces: Face[], side: number, width: number): Uint8Array {
	const height = width / 2;
	const rgb = new Uint8Array(width * height * 3);
	for (let y = 0; y < height; y++) {
		const lat = ((90 - ((y + 0.5) / height) * 180) * Math.PI) / 180;
		for (let x = 0; x < width; x++) {
			const lon = ((-180 + ((x + 0.5) / width) * 360) * Math.PI) / 180;
			const d = [Math.cos(lat) * Math.cos(lon), Math.sin(lat), Math.cos(lat) * Math.sin(lon)];
			const [sector, u, v] = fromDirection(d);
			const tx = Math.min(side - 1, Math.floor((u / SECTOR_SIDE) * side));
			const ty = Math.min(side - 1, Math.floor((v / SECTOR_SIDE) * side));
			const e = faces[sector].elevation[ty * side + tx];
			const at = (y * width + x) * 3;
			if (e < 0) {
				const deep = Math.min(1, -e / 6000);
				[rgb[at], rgb[at + 1], rgb[at + 2]] = [10, 30 + 40 * (1 - deep), 70 + 120 * (1 - deep)];
			} else {
				const up = Math.min(1, e / 4000);
				[rgb[at], rgb[at + 1], rgb[at + 2]] = [60 + 190 * up, 110 + 120 * up, 50 + 150 * up];
			}
		}
	}
	return png(rgb, width, height);
}

/** Adler-32 over the raw scanlines: what closes a zlib stream. */
function adler32(bytes: Uint8Array): number {
	let [a, b] = [1, 0];
	for (const byte of bytes) {
		a = (a + byte) % 65521;
		b = (b + a) % 65521;
	}
	return ((b << 16) | a) >>> 0;
}

/** Minimal PNG: one zlib stream, three chunks. */
function png(rgb: Uint8Array, width: number, height: number): Uint8Array {
	const raw = new Uint8Array((width * 3 + 1) * height);
	for (let y = 0; y < height; y++) {
		raw[y * (width * 3 + 1)] = 0;
		raw.set(rgb.subarray(y * width * 3, (y + 1) * width * 3), y * (width * 3 + 1) + 1);
	}
	const table = Array.from({ length: 256 }, (_, n) => {
		let c = n;
		for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
		return c >>> 0;
	});
	const crc = (bytes: Uint8Array) => {
		let c = 0xffffffff;
		for (const byte of bytes) c = table[(c ^ byte) & 0xff] ^ (c >>> 8);
		return (c ^ 0xffffffff) >>> 0;
	};
	const chunk = (kind: string, body: Uint8Array) => {
		const head = new Uint8Array(4 + body.length);
		head.set(new TextEncoder().encode(kind));
		head.set(body, 4);
		const out = new Uint8Array(8 + body.length + 4);
		new DataView(out.buffer).setUint32(0, body.length, false);
		out.set(head, 4);
		new DataView(out.buffer).setUint32(out.length - 4, crc(head), false);
		return out;
	};
	// `Bun.deflateSync` gives bare deflate; PNG wants it wrapped in zlib.
	const zlib = (body: Uint8Array) => {
		const packed = Bun.deflateSync(body);
		const out = new Uint8Array(2 + packed.length + 4);
		out.set([0x78, 0x01]);
		out.set(packed, 2);
		new DataView(out.buffer).setUint32(out.length - 4, adler32(body), false);
		return out;
	};
	const ihdr = new Uint8Array(13);
	const view = new DataView(ihdr.buffer);
	view.setUint32(0, width, false);
	view.setUint32(4, height, false);
	ihdr.set([8, 2, 0, 0, 0], 8);
	const parts = [
		new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
		chunk('IHDR', ihdr),
		chunk('IDAT', zlib(raw)),
		chunk('IEND', new Uint8Array(0))
	];
	const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
	let at = 0;
	for (const part of parts) {
		out.set(part, at);
		at += part.length;
	}
	return out;
}

// --- Run -------------------------------------------------------------------

await mkdir(CACHE, { recursive: true });
await mkdir(`${ROOT}/assets/fields`, { recursive: true });

console.log(`ETOPO 2022, ${COLS} x ${ROWS} samples`);
const faces = await scatter();
console.log(`\nbaking ${LEVELS} levels of ${SIDE} texels per face side`);

const levels: Face[][] = [faces];
for (let level = 1; level < LEVELS; level++) {
	levels.push(levels[level - 1].map((face) => halve(face, SIDE >> (level - 1))));
}

let bytes = 48;
for (let level = 0; level < LEVELS; level++) {
	const stride = (SIDE >> level) + 2;
	bytes += 6 * stride * stride * 3;
}
const out = new Uint8Array(bytes);
out.set(new TextEncoder().encode('PLFIELD1'));
const header = new DataView(out.buffer);
header.setUint32(40, SIDE, true);
header.setUint32(44, LEVELS, true);
let offset = 48;
for (let level = 0; level < LEVELS; level++) {
	offset = writeLevel(out, offset, levels[level], SIDE >> level);
}

// The id names the ground, so it covers everything but the id itself.
const hasher = new Bun.CryptoHasher('sha256');
hasher.update(out.subarray(40));
out.set(hasher.digest(), 8);
const id = Array.from(out.subarray(8, 40), (b) => b.toString(16).padStart(2, '0')).join('');

const path = `${ROOT}/assets/fields/earth.field`;
await Bun.write(path, out);
// The sidecar is how a page names the ground without downloading it first.
await Bun.write(
	`${ROOT}/assets/fields/earth.json`,
	JSON.stringify({ id, side: SIDE, levels: LEVELS, texel_m: (SECTOR_SIDE * 0.5) / SIDE, source: 'ETOPO 2022 (NOAA NCEI), public domain' }, null, '\t') + '\n'
);
await Bun.write(`${ROOT}/assets/fields/earth.png`, preview(faces, SIDE, 2048));
console.log(`\n${path}`);
console.log(`  ${(bytes / 1e6).toFixed(1)} MB, ${(SECTOR_SIDE * 0.5) / SIDE} m per texel`);
console.log(`  id ${id}`);

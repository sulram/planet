/**
 * What a world draws of one 3D model: the engine's budget
 * (`crates/client/src/model3d.rs`), said here for the words that refuse a
 * file over it; `bun run docs` holds the two to the same numbers. The engine
 * weighs a file (`Seam.weigh`); these only word its answer.
 */
import { n } from '$lib/i18n';

/** Triangles drawn, a face seen from both sides counting twice. */
export const MAX_TRIANGLES = 65536;
/** Texels of all its pictures together. */
export const MAX_TEXELS = 4194304;
export const MAX_MATERIALS = 32;
/** Vertices its meshes hold, each mesh counted once. */
export const MAX_VERTICES = 196608;

/**
 * The numbers a refusal of a 3D model says, as its words name them, in the
 * reader's locale. A picture's side is a size, written whole: 2048.
 */
export const budget = () => ({
	triangles: n(MAX_TRIANGLES),
	side: String(Math.sqrt(MAX_TEXELS)),
	materials: n(MAX_MATERIALS),
	vertices: n(MAX_VERTICES)
});

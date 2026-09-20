import { readFile } from 'node:fs/promises';
import { FIELD_PATH, type Field, type Shape } from '$lib/world';

// Baked fields are served from `static/assets/fields`, which is a link to the
// instance's asset set. The sidecar beside each one names the ground, so a
// page can build a whole recipe without downloading a texel.

const SIDECAR: Record<Exclude<Shape, 'generated'>, string> = {
	earth: 'earth.json'
};

/** Where the field a recipe names is served from, or null when this instance
 *  does not have that ground. Looked up by content id, never by name: a world
 *  is only ever the world its field was baked for. */
export async function servedField(id: string | null): Promise<string | null> {
	if (!id) return null;
	for (const shape of Object.keys(SIDECAR) as Exclude<Shape, 'generated'>[]) {
		const field = await readFieldSidecar(shape);
		if (field?.id === id) return FIELD_PATH[shape];
	}
	return null;
}

/** Null when this instance has no such field: `bun run field` bakes them. */
export async function readFieldSidecar(shape: Exclude<Shape, 'generated'>): Promise<Field | null> {
	try {
		const url = new URL(`../../../static/assets/fields/${SIDECAR[shape]}`, import.meta.url);
		const field = JSON.parse(await readFile(url, 'utf8')) as Field;
		return field.id ? field : null;
	} catch {
		// Not baked here, or unreadable: the page offers the generated shape.
		return null;
	}
}

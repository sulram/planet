import { FIELD_PATH, fieldUrl, type Baked, type Field } from '$lib/world';

// Baked fields are served under `/assets/fields`, beside the rest of the
// asset set. The sidecar beside each field names the ground, so a page builds
// a whole recipe without downloading a texel.

/** The sidecar of a field is the `.json` beside its `.field`. */
function sidecarPath(shape: Baked): string {
	return FIELD_PATH[shape].replace(/\.field$/, '.json');
}

/** Null when this version has no such field: `bun run field` bakes them. */
export async function readFieldSidecar(shape: Baked): Promise<Field | null> {
	try {
		const response = await fetch(sidecarPath(shape));
		if (!response.ok) return null;
		const field = (await response.json()) as Field;
		return field.id ? field : null;
	} catch {
		// Not baked here, or unreadable: the page offers the generated shape.
		return null;
	}
}

/**
 * The URL the field a recipe names is fetched from, or null when this version
 * does not have that ground. Looked up by content id, never by name: a world
 * is only ever the world its field was baked for.
 */
export async function servedField(id: string | null): Promise<string | null> {
	if (!id) return null;
	for (const shape of Object.keys(FIELD_PATH) as Baked[]) {
		const field = await readFieldSidecar(shape);
		if (field?.id === id) return fieldUrl(shape, id);
	}
	return null;
}

import { FIELD_PATH, type Field, type Shape } from '$lib/world';

// Baked fields are served under `/assets/fields`, the instance's asset set,
// and read the way the avatar manifest is: through the app's own `fetch`,
// which during server rendering answers a relative path from the static files
// without a network call and without caring where the built code sits. The
// sidecar beside each field names the ground, so a page can build a whole
// recipe without downloading a texel.

type Fetch = typeof globalThis.fetch;
type Baked = Exclude<Shape, 'generated'>;

/** The sidecar of a field is the `.json` beside its `.field`. */
function sidecarPath(shape: Baked): string {
	return FIELD_PATH[shape].replace(/\.field$/, '.json');
}

/** Where the field a recipe names is served from, or null when this instance
 *  does not have that ground. Looked up by content id, never by name: a world
 *  is only ever the world its field was baked for. */
export async function servedField(fetch: Fetch, id: string | null): Promise<string | null> {
	if (!id) return null;
	for (const shape of Object.keys(FIELD_PATH) as Baked[]) {
		const field = await readFieldSidecar(fetch, shape);
		if (field?.id === id) return FIELD_PATH[shape];
	}
	return null;
}

/** Null when this instance has no such field: `bun run field` bakes them. */
export async function readFieldSidecar(fetch: Fetch, shape: Baked): Promise<Field | null> {
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

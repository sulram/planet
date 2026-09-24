import { error, json } from '@sveltejs/kit';
import { chooseName } from '$lib/server/name';
import type { RequestHandler } from './$types';

// The person chose what to be called; this keeps it for their next visit.
export const POST: RequestHandler = async (event) => {
	const body: unknown = await event.request.json().catch(() => null);
	const name = (body as { name?: unknown } | null)?.name;
	if (typeof name !== 'string') error(400);
	return json({ name: await chooseName(event, name) });
};

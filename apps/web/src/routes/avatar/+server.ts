import { error } from '@sveltejs/kit';
import { chooseAvatar } from '$lib/server/avatar';
import type { RequestHandler } from './$types';

// The engine reports the avatar now worn; this keeps it as the visitor's choice.
export const POST: RequestHandler = async (event) => {
	const body: unknown = await event.request.json().catch(() => null);
	const path = (body as { path?: unknown } | null)?.path;
	if (typeof path !== 'string' || !(await chooseAvatar(event, path))) error(400);
	return new Response(null, { status: 204 });
};

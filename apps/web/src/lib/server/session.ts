import type PocketBase from 'pocketbase';
import { PB_PUBLIC_URL } from './pb';

/**
 * The hot plane's doors, as the page hands them to the engine. The engine
 * opens the socket itself; the page only tells it where and, for a signed in
 * person, hands it a ticket minted with their own token from the server side,
 * so the session cookie never reaches a script.
 */
export interface Link {
	/** The world's socket, without a ticket. */
	url: string;
	/** Where the page fetches a fresh ticket before each connection; null for a visitor. */
	ticketPath: string | null;
}

export function worldLink(worldId: string, signedIn: boolean): Link {
	const base = PB_PUBLIC_URL.replace(/^http/, 'ws');
	return {
		url: `${base}/api/planet/worlds/${encodeURIComponent(worldId)}/socket`,
		ticketPath: signedIn ? `/w/${encodeURIComponent(worldId)}/ticket` : null
	};
}

/** A ticket for the socket, worth one connection for a minute. */
export async function mintTicket(pb: PocketBase): Promise<string> {
	const res = await pb.send<{ ticket: string }>('/api/planet/ticket', { method: 'POST' });
	return res.ticket;
}

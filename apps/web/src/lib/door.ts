/**
 * mundos's door (DECISIONS 87). Every load passes through it: a page that
 * arrives with no word from the door goes there, and comes back with a token
 * mundos signed, or as a guest, in the address's fragment, which browsers
 * keep out of requests and logs.
 */

const WORD = /^#identity=(.+)$/;
/** Where the address waits while the page is at the door, which sends everyone back to `/`. */
const KEPT = 'planet.address';

/** What the door said, and the address the page had before it went there. */
export interface Arrival {
	/** A token mundos signed, or `guest`. */
	identity: string;
	/** Where to go back to: the query and the place the page left with. */
	address: string;
}

/** What the door left in the address, or null for a page yet to pass through it. */
export function arrival(): Arrival | null {
	const word = WORD.exec(location.hash);
	if (!word) return null;
	let before = '';
	try {
		before = sessionStorage.getItem(KEPT) ?? '';
		sessionStorage.removeItem(KEPT);
	} catch {
		// storage blocked: the page comes back to the world's own address
	}
	return { identity: decodeURIComponent(word[1]), address: `/${before}` };
}

/**
 * Goes to the door, keeping the address for the way back: a place in a link
 * survives the sign in. The page is left here.
 */
export function pass(door: string): void {
	try {
		sessionStorage.setItem(KEPT, location.search + location.hash);
	} catch {
		// storage blocked: the entry goes on, from the world's own address
	}
	location.replace(`${door}?host=${encodeURIComponent(location.host)}`);
}

/**
 * What this browser keeps of a person's choices: the avatar worn, a visitor's
 * name. A convenience: a browser that keeps them spares the person choosing
 * again.
 */
const PREFIX = 'planet.';

export function kept(name: string): string | null {
	try {
		return localStorage.getItem(PREFIX + name);
	} catch {
		// storage blocked: the person chooses again
		return null;
	}
}

export function keep(name: string, value: string): void {
	try {
		localStorage.setItem(PREFIX + name, value);
	} catch {
		// best effort: the choice then lasts for this visit
	}
}

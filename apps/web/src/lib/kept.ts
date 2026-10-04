/**
 * What this browser keeps of a person's choices: the avatar worn, a visitor's
 * name. Nothing a world needs: a browser that keeps nothing loses only the
 * choice.
 */
const PREFIX = 'planet.';

export function kept(name: string): string | null {
	try {
		return localStorage.getItem(PREFIX + name);
	} catch {
		// storage blocked: the same as nothing kept
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

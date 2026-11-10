import { beforeEach, describe, expect, test } from 'bun:test';
import { wornAvatar } from './avatar';
import { keep, kept } from './kept';

const offer = ['avatars/Kyle.vrm', 'avatars/Skull.vrm'];
const own = 'https://mundos-assets.tekne.studio/avatars/av1/0123abcd.vrm';

// The browser's storage, as a page has it.
beforeEach(() => {
	const held = new Map<string, string>();
	globalThis.localStorage = {
		getItem: (name: string) => held.get(name) ?? null,
		setItem: (name: string, value: string) => void held.set(name, value)
	} as Storage;
});

describe('wornAvatar', () => {
	test("wears the account's own and keeps nothing of it", () => {
		keep('avatar', 'avatars/Skull.vrm');
		expect(wornAvatar(own, offer)).toBe(own);
		expect(kept('avatar')).toBe('avatars/Skull.vrm');
	});

	test('with none, wears what this browser kept while it is on offer', () => {
		keep('avatar', 'avatars/Skull.vrm');
		expect(wornAvatar('', offer)).toBe('avatars/Skull.vrm');
	});

	test('else picks one from the offer and keeps it', () => {
		keep('avatar', 'avatars/Gone.vrm');
		const picked = wornAvatar('', offer);
		expect(offer).toContain(picked!);
		expect(kept('avatar')).toBe(picked);
	});

	test('and with nothing offered leaves it to the engine', () => {
		expect(wornAvatar('', [])).toBeNull();
	});
});

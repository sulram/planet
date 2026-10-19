import { describe, expect, test } from 'bun:test';
import { assetUrl, isAsset, kindOf } from './files';

const bytes = (text: string, ...rest: number[]) => new Uint8Array([...[...text].map((c) => c.charCodeAt(0)), ...rest]);
const hash = '0'.repeat(64);

describe('kindOf', () => {
	test('reads the kinds the world takes off their first bytes', () => {
		expect(kindOf(bytes('VOX ', 150, 0, 0, 0))).toBe('vox');
		expect(kindOf(new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 1]))).toBe('png');
		expect(kindOf(new Uint8Array([0xff, 0xd8, 0xff, 0xe0]))).toBe('jpg');
		expect(kindOf(bytes('RIFF\0\0\0\0WEBPVP8 '))).toBe('webp');
		expect(kindOf(bytes('GIF89a\x10\0\x10\0'))).toBe('gif');
		expect(kindOf(bytes('\0\0\0\x18ftypisom'))).toBe('mp4');
		expect(kindOf(bytes('\0\0\0\x18ftypM4A '))).toBe('m4a');
		expect(kindOf(bytes('ID3\x04\0\0'))).toBe('mp3');
		expect(kindOf(new Uint8Array([0xff, 0xfb, 0x90, 0x00]))).toBe('mp3');
		expect(kindOf(bytes('OggS\0\x02'))).toBe('ogg');
	});

	test('and nothing else', () => {
		expect(kindOf(bytes('glTF'))).toBeNull();
		expect(kindOf(bytes('RIFF\0\0\0\0WAVEfmt '))).toBeNull();
		expect(kindOf(bytes('VOX'))).toBeNull();
		expect(kindOf(new Uint8Array())).toBeNull();
	});
});

describe('isAsset', () => {
	test('is a hash and a kind after asset://', () => {
		expect(isAsset(`asset://${hash}.vox`)).toBe(true);
		expect(isAsset(`asset://${hash}.webp`)).toBe(true);
		expect(isAsset(`asset://${hash}`)).toBe(false);
		expect(isAsset(`asset://../world.json`)).toBe(false);
		expect(isAsset(`${hash}.vox`)).toBe(false);
		expect(isAsset(42)).toBe(false);
	});
});

describe('assetUrl', () => {
	test('puts the name under the base the world names', () => {
		expect(assetUrl(`asset://${hash}.vox`, '/files')).toBe(`/files/${hash}.vox`);
		expect(assetUrl(`asset://${hash}.png`, 'https://cdn.example/w1/')).toBe(`https://cdn.example/w1/${hash}.png`);
	});
});

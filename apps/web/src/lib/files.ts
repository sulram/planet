/**
 * The world's files (docs/ARCHITECTURE.md § Files): a heavy file sent to the
 * world, which names it by its content, and read back by that name. The
 * routes are `server/internal/api/upload.go`.
 */
import { Refused } from './instance';

/** What a kept file is called in a record: `asset://<sha256>.<ext>`. */
export type Asset = `asset://${string}`;

/** The kinds the world takes, each read off the first bytes and never off a name. */
export type Kind = 'vox' | 'png' | 'jpg' | 'webp';

/** The kind of some bytes, as the server reads it. Null for bytes of no kind the world takes. */
export function kindOf(head: Uint8Array): Kind | null {
	const starts = (bytes: number[], at = 0) => bytes.every((byte, i) => head[at + i] === byte);
	const ascii = (text: string, at = 0) => starts([...text].map((c) => c.charCodeAt(0)), at);
	if (ascii('VOX ')) return 'vox';
	if (starts([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])) return 'png';
	if (starts([0xff, 0xd8, 0xff])) return 'jpg';
	if (head.length >= 12 && ascii('RIFF') && ascii('WEBP', 8)) return 'webp';
	return null;
}

/** Whether a string is an asset's name. */
export const isAsset = (name: unknown): name is Asset =>
	typeof name === 'string' && /^asset:\/\/[0-9a-f]{64}\.[a-z0-9]{2,5}$/.test(name);

/**
 * Sends a file to the world, which names it by its content. A builder's and
 * an admin's: the key says who. Throws a `Refused` with the server's code:
 * `level`, `kind`, `size`, or `unreachable`.
 */
export async function upload(file: Blob, key: string): Promise<Asset> {
	let response: Response;
	try {
		response = await fetch('/api/files', {
			method: 'POST',
			headers: key ? { authorization: `Bearer ${key}` } : {},
			body: file
		});
	} catch {
		throw new Refused('unreachable');
	}
	const body: unknown = await response.json().catch(() => null);
	if (!response.ok) {
		const code = (body as { error?: unknown } | null)?.error;
		throw new Refused(typeof code === 'string' ? code : 'unreachable');
	}
	const asset = (body as { asset?: unknown } | null)?.asset;
	if (!isAsset(asset)) throw new Refused('unreachable');
	return asset;
}

/** Where an asset is read from: `asset://x` under the base the world's description names. */
export function assetUrl(asset: Asset, files: string): string {
	return `${files.replace(/\/$/, '')}/${asset.slice('asset://'.length)}`;
}

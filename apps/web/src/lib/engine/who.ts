import { t } from '$lib/i18n';
import type { PeerInfo } from './protocol';

/**
 * How a peer is named on screen: by name, or told apart by the session
 * number the world gave them on arrival.
 */
export function who(peer: PeerInfo): string {
	if (peer.name) return peer.name;
	return t(peer.visitor ? 'engine.here.visitor' : 'engine.here.someone', { n: peer.session });
}

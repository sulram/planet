/**
 * The kinds of item the menu offers, in order. Each is the name the seam
 * knows it by, its icon and its label, and, when picking it needs more than
 * a click, a component mounted in the panel while it is in hand: a file to
 * choose, a size to say. A plugin's session that adds a kind adds its row
 * here and nothing else of the panel.
 */
import type { Component } from 'svelte';
import type { IconName } from '$lib/ds';
import type { Seam } from '$lib/plugins/plugin';
import Picture from './Picture.svelte';
import Model from './Model.svelte';
import type { Item } from './protocol';
import { t } from './words';

/** What a kind's own component is mounted with. */
export interface KindProps {
	seam: Seam;
}

export interface Kind {
	name: Item;
	icon: IconName;
	label: () => string;
	/** Its own controls, while it is in hand. */
	Extra?: Component<KindProps>;
}

export const kinds: readonly Kind[] = [
	{ name: 'cube', icon: 'box', label: () => t('kind.cube') },
	{ name: 'picture', icon: 'image', label: () => t('kind.picture'), Extra: Picture },
	{ name: 'model', icon: 'package', label: () => t('kind.model'), Extra: Model }
];

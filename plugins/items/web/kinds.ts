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
import Video from './Video.svelte';
import Model from './Model.svelte';
import Landmark from './Landmark.svelte';
import Sound from './Sound.svelte';
import Portal from './Portal.svelte';
import type { Item, Picked } from './protocol';
import { t } from './words';

/** What a kind's own component is mounted with. */
export interface KindProps {
	seam: Seam;
	/** The part picked, while it is of this kind: the controls act on it instead of on what is in hand. */
	picked?: Picked;
}

export interface Kind {
	name: Item;
	icon: IconName;
	label: () => string;
	/** Its own controls, while it is in hand. */
	Extra?: Component<KindProps>;
	/** Whether it is placed by a button too, in front of the body with no click: a kind that needs no file. A file given places itself so. */
	adds?: boolean;
}

export const kinds: readonly Kind[] = [
	{ name: 'cube', icon: 'box', label: () => t('kind.cube'), adds: true },
	{ name: 'picture', icon: 'image', label: () => t('kind.picture'), Extra: Picture },
	{ name: 'video', icon: 'video', label: () => t('kind.video'), Extra: Video },
	{ name: 'model', icon: 'package', label: () => t('kind.model'), Extra: Model },
	{ name: 'sound', icon: 'volume-2', label: () => t('kind.sound'), Extra: Sound },
	{ name: 'landmark', icon: 'star', label: () => t('kind.landmark'), Extra: Landmark, adds: true },
	{ name: 'portal', icon: 'circle', label: () => t('kind.portal'), Extra: Portal, adds: true }
];

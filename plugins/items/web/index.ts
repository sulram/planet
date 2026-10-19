import type { WebPlugin } from '$lib/plugins/plugin';
import Layer from './Layer.svelte';
import { t } from './words';

/** Items in the web front end: the menu in the top left corner of things to place. */
const items: WebPlugin = {
	name: 'items',
	label: () => t('name'),
	Layer,
	hints: [
		{ keys: 'I', does: () => t('hint.items') },
		{ keys: () => t('hint.place.keys'), does: () => t('hint.place') }
	]
};

export default items;

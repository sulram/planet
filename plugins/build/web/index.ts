import type { WebPlugin } from '$lib/plugins/plugin';
import Layer from './Layer.svelte';
import { t } from './words';

/** Building in the web front end: a button in the corner, and the panel of the tool in hand. */
const build: WebPlugin = {
	name: 'build',
	label: () => t('name'),
	Layer,
	hints: [
		{ keys: 'B', does: () => t('hint.build') },
		{ keys: '1 2 3', does: () => t('hint.tools') },
		{ keys: () => t('hint.look.keys'), does: () => t('hint.look') },
		{ keys: () => t('hint.turn.keys'), does: () => t('hint.turn') },
		{ keys: () => t('hint.undo.keys'), does: () => t('hint.undo') },
		{ keys: 'Esc', does: () => t('hint.cancel') }
	]
};

export default build;

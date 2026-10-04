import type { WebPlugin } from '$lib/plugins/plugin';
import Layer from './Layer.svelte';

/** Building in the web front end: a button in the corner, and the panel of the tool in hand. */
const build: WebPlugin = {
	name: 'build',
	version: 1,
	label: 'engine.plugin.build',
	Layer,
	hints: [
		{ keys: 'B', does: 'engine.hint.build' },
		{ keys: '1 2 3', does: 'engine.hint.tools' },
		{ keys: { message: 'engine.hint.lookBuilding.keys' }, does: 'engine.hint.lookBuilding' },
		{ keys: { message: 'engine.hint.upright.keys' }, does: 'engine.hint.upright' },
		{ keys: { message: 'engine.hint.undo.keys' }, does: 'engine.hint.undo' },
		{ keys: 'Esc', does: 'engine.hint.cancel' }
	]
};

export default build;

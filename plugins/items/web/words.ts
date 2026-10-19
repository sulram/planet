import { words } from '$lib/i18n';

// Items' own words, in each language the front end speaks.
// Copy rules: short, professional, no em dashes, no internal information.
const en = {
	name: 'Items',
	title: 'Items',
	kind: 'Item',
	'kind.cube': 'Cube',
	paint: 'Colour',
	'paint.pick': 'Colour {n}',
	material: 'Material',
	finish: 'Finish',
	'finish.matte': 'Matte',
	'finish.glass': 'Glass',
	'finish.light': 'Light',
	edge: 'Edge',
	'edge.none': 'None',
	'edge.black': 'Black',
	'edge.white': 'White',
	'help.pick': 'Pick an item to place.',
	'help.place': 'Click where it goes: on the ground, on a side or on another item. Esc puts it down.',
	'refused.level': 'Only builders of this world can place items here.',
	'refused.pose': 'That size or turn is not possible.',
	'refused.gone': 'That item is gone.',
	'hint.items': 'items, or put the item down',
	'hint.place.keys': 'Click',
	'hint.place': 'place the item in hand'
};

const pt: typeof en = {
	name: 'Itens',
	title: 'Itens',
	kind: 'Item',
	'kind.cube': 'Cubo',
	paint: 'Cor',
	'paint.pick': 'Cor {n}',
	material: 'Material',
	finish: 'Acabamento',
	'finish.matte': 'Fosco',
	'finish.glass': 'Vidro',
	'finish.light': 'Luz',
	edge: 'Borda',
	'edge.none': 'Nenhuma',
	'edge.black': 'Preta',
	'edge.white': 'Branca',
	'help.pick': 'Escolha um item para colocar.',
	'help.place': 'Clique onde ele vai: no chão, numa lateral ou sobre outro item. Esc larga o item.',
	'refused.level': 'Só construtores deste mundo podem colocar itens aqui.',
	'refused.pose': 'Esse tamanho ou giro não é possível.',
	'refused.gone': 'Esse item não existe mais.',
	'hint.items': 'itens, ou largar o item',
	'hint.place.keys': 'Clique',
	'hint.place': 'colocar o item na mão'
};

const zh: typeof en = {
	name: '物件',
	title: '物件',
	kind: '物件',
	'kind.cube': '立方体',
	paint: '颜色',
	'paint.pick': '颜色 {n}',
	material: '材质',
	finish: '质感',
	'finish.matte': '哑光',
	'finish.glass': '玻璃',
	'finish.light': '发光',
	edge: '边线',
	'edge.none': '无',
	'edge.black': '黑',
	'edge.white': '白',
	'help.pick': '选择要放置的物件。',
	'help.place': '点击放置位置：地面、侧面或另一个物件上。Esc 放下。',
	'refused.level': '只有本世界的建造者可以在这里放置物件。',
	'refused.pose': '无法使用该大小或旋转。',
	'refused.gone': '该物件已不存在。',
	'hint.items': '物件，或放下手中的物件',
	'hint.place.keys': '点击',
	'hint.place': '放置手中的物件'
};

export const t = words({ en, pt, zh });

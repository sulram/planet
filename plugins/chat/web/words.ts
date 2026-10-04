import { words } from '$lib/i18n';

// Chat's own words, in each language the front end speaks.
// Copy rules: short, professional, no em dashes, no internal information.
const en = {
	name: 'Chat',
	title: 'Chat',
	near: 'Near',
	world: 'World',
	scope: 'Who hears you',
	placeholder: 'Say something. {here} shares where you stand.',
	offline: 'Chat opens when you are online.',
	send: 'Send',
	closed: 'Enter to chat',
	left: '{n} characters left',
	here: '@here',
	goto: 'Go to {place}',
	lines: 'Lines said',
	'hint.chat': 'chat'
};

const pt: typeof en = {
	name: 'Chat',
	title: 'Conversa',
	near: 'Perto',
	world: 'Mundo',
	scope: 'Quem ouve você',
	placeholder: 'Diga algo. {here} compartilha onde você está.',
	offline: 'A conversa abre quando você está conectado.',
	send: 'Enviar',
	closed: 'Enter para conversar',
	left: '{n} caracteres restantes',
	here: '@aqui',
	goto: 'Ir para {place}',
	lines: 'Linhas ditas',
	'hint.chat': 'conversar'
};

export const t = words({ en, pt });

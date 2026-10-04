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

const zh: typeof en = {
	name: '聊天',
	title: '聊天',
	near: '附近',
	world: '世界',
	scope: '谁能听到你',
	placeholder: '说点什么。{here} 会分享你所在的位置。',
	offline: '上线后即可聊天。',
	send: '发送',
	closed: '按 Enter 聊天',
	left: '还剩 {n} 个字符',
	here: '@这里',
	goto: '前往 {place}',
	lines: '聊天记录',
	'hint.chat': '聊天'
};

export const t = words({ en, pt, zh });

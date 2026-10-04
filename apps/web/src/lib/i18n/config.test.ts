import { describe, expect, test } from 'bun:test';
import { htmlLang, locales, resolveLocale, translate } from './config';

describe('the languages the page speaks', () => {
	test('are the ones mundos speaks, by the same codes', () => {
		expect([...locales].sort()).toEqual(['en', 'pt', 'zh']);
		expect(htmlLang.zh).toBe('zh-CN');
	});

	test('a choice kept wins, then the browser by base language, then the default', () => {
		expect(resolveLocale('zh', 'pt-BR,pt;q=0.9')).toBe('zh');
		expect(resolveLocale(undefined, 'zh-TW,zh;q=0.9,en;q=0.8')).toBe('zh');
		expect(resolveLocale('xx', 'fr-FR,pt;q=0.5')).toBe('pt');
		expect(resolveLocale(undefined, 'fr-FR')).toBe('en');
	});

	test('every one fills a message in', () => {
		for (const locale of locales) {
			const page = translate(locale, 'common.pageOf', { page: 2, pages: 9 });
			expect(page).toContain('2');
			expect(page).toContain('9');
			expect(page).not.toContain('{');
		}
	});
});

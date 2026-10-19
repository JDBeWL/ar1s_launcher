import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

import { cache } from '../services/cache';
import { DEFAULT_LOCALE, currentLocale, i18n, setLocale } from '../i18n';
import { UI_LANGUAGE_CONFIG_KEY, useUiLanguageStore } from './uiLanguage';

describe('UiLanguageStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    cache.clear();
    invokeMock.mockReset();
    setLocale(DEFAULT_LOCALE);
  });

  it('配置文件里有合法语言时以它为准', async () => {
    invokeMock.mockImplementation((command: string, args?: { key?: string }) => {
      if (command === 'load_config_key' && args?.key === UI_LANGUAGE_CONFIG_KEY) return Promise.resolve('en-US');
      return Promise.resolve(undefined);
    });

    const store = useUiLanguageStore();
    await store.load();

    expect(store.current).toBe('en-US');
    expect(store.isChinese).toBe(false);
    expect(currentLocale()).toBe('en-US');
  });

  it('配置文件里没有时沿用推断值并落盘', async () => {
    invokeMock.mockResolvedValue(null);

    const store = useUiLanguageStore();
    const initial = store.current;
    await store.load();

    expect(store.current).toBe(initial);
    expect(invokeMock).toHaveBeenCalledWith('save_config_key', {
      key: UI_LANGUAGE_CONFIG_KEY,
      value: initial,
    });
  });

  it('切换语言会写配置并更新 i18n', async () => {
    invokeMock.mockResolvedValue(undefined);

    const store = useUiLanguageStore();
    await store.setLanguage('en-US');

    expect(store.current).toBe('en-US');
    expect(currentLocale()).toBe('en-US');
    expect(i18n.global.locale.value).toBe('en-US');
    expect(invokeMock).toHaveBeenCalledWith('save_config_key', {
      key: UI_LANGUAGE_CONFIG_KEY,
      value: 'en-US',
    });
  });

  it('切到同一语言时不重复写配置', async () => {
    invokeMock.mockResolvedValue(undefined);

    const store = useUiLanguageStore();
    await store.setLanguage(store.current);

    expect(invokeMock).not.toHaveBeenCalled();
  });

  it('写配置失败时语言仍然生效', async () => {
    invokeMock.mockRejectedValue({ code: 'io', message: '写入失败' });

    const store = useUiLanguageStore();
    await store.setLanguage('en-US');

    expect(store.current).toBe('en-US');
    expect(currentLocale()).toBe('en-US');
  });

  it('语言列表用各自语言书写', () => {
    const store = useUiLanguageStore();

    expect(store.languages.map((item) => item.value)).toEqual(['zh-CN', 'en-US']);
    expect(store.languages.map((item) => item.nativeName)).toEqual(['简体中文', 'English']);
  });
});

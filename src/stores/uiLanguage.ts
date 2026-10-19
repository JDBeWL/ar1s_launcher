/**
 * 界面语言状态
 *
 * 语言以配置文件里的 uiLanguage 为准，localStorage 只当首屏缓存；
 * 切语言时同时更新 i18n、<html lang>、localStorage 与配置。
 */
import { computed, ref } from 'vue';
import { defineStore } from 'pinia';
import { configApi } from '../services';
import { DEFAULT_LOCALE, detectLocale, normalizeLocale, setLocale, type UiLocale } from '../i18n';
import { logError } from '../utils/logger';

/** 配置里存界面语言的键名，与 Rust 的 ConfigKey 对应 */
export const UI_LANGUAGE_CONFIG_KEY = 'uiLanguage';

export interface UiLanguageOption {
  value: UiLocale;
  /** 用各自语言书写，切到别的语言时这一项也不会变成看不懂的文字 */
  nativeName: string;
}

export const UI_LANGUAGES: ReadonlyArray<UiLanguageOption> = [
  { value: 'zh-CN', nativeName: '简体中文' },
  { value: 'en-US', nativeName: 'English' },
];

export const useUiLanguageStore = defineStore('uiLanguage', () => {
  const current = ref<UiLocale>(detectLocale());
  const languages = UI_LANGUAGES;
  const defaultLocale = DEFAULT_LOCALE;

  const isChinese = computed(() => current.value === 'zh-CN');

  function apply(next: UiLocale) {
    current.value = next;
    setLocale(next);
  }

  /** 首屏：先用缓存/系统语言，再以配置文件为准 */
  async function load() {
    try {
      const stored = await configApi.loadConfigKey(UI_LANGUAGE_CONFIG_KEY);
      const normalized = normalizeLocale(stored);
      if (normalized) {
        apply(normalized);
        return;
      }
      // 配置里没有：沿用首屏推断值，顺手落盘，免得每次启动都重新猜
      await configApi.saveConfigKey(UI_LANGUAGE_CONFIG_KEY, current.value);
    } catch (err) {
      logError('Failed to load UI language', err, 'UiLanguageStore');
    }
  }

  async function setLanguage(next: UiLocale) {
    if (next === current.value) return;
    apply(next);
    try {
      await configApi.saveConfigKey(UI_LANGUAGE_CONFIG_KEY, next);
    } catch (err) {
      logError('Failed to save UI language', err, 'UiLanguageStore');
    }
  }

  return {
    current,
    languages,
    defaultLocale,
    isChinese,
    load,
    setLanguage,
  };
});

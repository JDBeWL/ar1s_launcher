import { afterEach, describe, expect, it } from 'vitest';
import zhCN from './locales/zh-CN';
import enUS from './locales/en-US';
import { DEFAULT_LOCALE, SUPPORTED_LOCALES, normalizeLocale, setLocale, translate } from './index';

type Tree = { [key: string]: string | Tree };

/** 把嵌套消息树压平成 a.b.c，方便比对两种语言的键集合 */
function flatten(tree: Tree, prefix = ''): Map<string, string> {
  const out = new Map<string, string>();
  for (const [key, value] of Object.entries(tree)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (typeof value === 'string') {
      out.set(path, value);
    } else {
      for (const [nested, text] of flatten(value, path)) {
        out.set(nested, text);
      }
    }
  }
  return out;
}

const zhKeys = flatten(zhCN as Tree);
const enKeys = flatten(enUS as Tree);

/**
 * 源码文本按需读入
 *
 * 用 Vite 的 `?raw` 而不是 node:fs：前端工程没装 @types/node，
 * 走打包器的资源导入既能过类型检查，也能在浏览器环境里跑。
 */
const sourceFiles = import.meta.glob<string>(['../**/*.{vue,ts}', '!../i18n/**'], {
  query: '?raw',
  import: 'default',
  eager: true,
});

function keysUsedInSource(patterns: RegExp[]): Set<string> {
  const used = new Set<string>();
  for (const [file, source] of Object.entries(sourceFiles)) {
    // 测试文件里的中文是断言用的字面量，不是界面文案
    if (file.endsWith('.test.ts')) continue;
    for (const pattern of patterns) {
      for (const match of source.matchAll(pattern)) {
        used.add(match[1]);
      }
    }
  }
  return used;
}

describe('i18n 消息完整性', () => {
  it('两种语言的键完全一致', () => {
    const missingInEn = [...zhKeys.keys()].filter((key) => !enKeys.has(key));
    const missingInZh = [...enKeys.keys()].filter((key) => !zhKeys.has(key));

    expect(missingInEn).toEqual([]);
    expect(missingInZh).toEqual([]);
  });

  it('没有空文案', () => {
    for (const [key, value] of [...zhKeys, ...enKeys]) {
      expect(value.trim(), key).not.toBe('');
    }
  });

  it('脚本里 t() / translate() 用到的字面量键都存在', () => {
    // 拼接出来的键（如 theme.presets.${id}）无法静态检查，只统计字面量
    const used = keysUsedInSource([/\b(?:t|translate)\(\s*'([a-zA-Z][\w.]*)'/g]);

    expect(used.size).toBeGreaterThan(100);
    expect([...used].filter((key) => !zhKeys.has(key))).toEqual([]);
  });

  it('模板里的 $t 与 keypath 也都能解析', () => {
    const used = keysUsedInSource([/\$t\(\s*'([a-zA-Z][\w.]*)'/g, /keypath="([a-zA-Z][\w.]*)"/g]);

    expect(used.size).toBeGreaterThan(0);
    expect([...used].filter((key) => !zhKeys.has(key))).toEqual([]);
  });
});

describe('Vuetify 组件文案', () => {
  it('语言包挂在 $vuetify 下，组件查得到翻译', () => {
    // Vuetify 内部用 `$vuetify.xxx` 查文案；平铺到根上就会显示原始 key
    expect(zhKeys.has('$vuetify.dataFooter.itemsPerPageAll')).toBe(true);
    expect(enKeys.has('$vuetify.dataFooter.itemsPerPageAll')).toBe(true);
    expect(zhKeys.get('$vuetify.pagination.ariaLabel.root')).toBe('分页导航');
    expect(enKeys.get('$vuetify.pagination.ariaLabel.root')).toBe('Pagination Navigation');
  });

  it('翻译函数能取到 Vuetify 的键', () => {
    expect(translate('$vuetify.dataFooter.itemsPerPageAll')).toBe(zhKeys.get('$vuetify.dataFooter.itemsPerPageAll'));
  });
});

describe('语言解析', () => {
  it('把常见写法收敛到受支持的两种', () => {
    expect(normalizeLocale('zh')).toBe('zh-CN');
    expect(normalizeLocale('zh-Hans')).toBe('zh-CN');
    expect(normalizeLocale('zh_CN')).toBe('zh-CN');
    expect(normalizeLocale('zh-TW')).toBe('zh-CN');
    expect(normalizeLocale('en')).toBe('en-US');
    expect(normalizeLocale('en-GB')).toBe('en-US');
    expect(normalizeLocale('  EN_us  ')).toBe('en-US');
    expect(normalizeLocale('fr-FR')).toBeNull();
    expect(normalizeLocale(null)).toBeNull();
    expect(normalizeLocale('')).toBeNull();
  });

  it('受支持语言列表与默认语言一致', () => {
    expect(SUPPORTED_LOCALES).toContain(DEFAULT_LOCALE);
    expect(new Set(SUPPORTED_LOCALES).size).toBe(SUPPORTED_LOCALES.length);
  });
});

describe('translate()', () => {
  afterEach(() => {
    // 语言是全局状态，用完复位，避免影响后面的用例
    setLocale(DEFAULT_LOCALE);
  });

  it('按当前语言返回文案，并支持具名插值', () => {
    setLocale('zh-CN');
    expect(translate('common.cancel')).toBe(zhKeys.get('common.cancel'));
    expect(translate('time.minutesAgo', { count: 5 }, 5)).toContain('5');
  });

  it('切到英文后返回英文文案', () => {
    setLocale('en-US');
    expect(translate('common.cancel')).toBe(enKeys.get('common.cancel'));
    expect(translate('common.cancel')).not.toBe(zhKeys.get('common.cancel'));
  });

  it('缺失的键原样返回，不会抛错', () => {
    setLocale('zh-CN');
    expect(translate('does.not.exist')).toBe('does.not.exist');
  });
});

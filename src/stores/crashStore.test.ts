/**
 * 崩溃诊断弹窗状态测试
 * 每次崩溃覆盖上一份，否则第二次崩的结论会混着第一次的 findings 显示
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { useCrashStore } from './crashStore';
import type { CrashDiagnosis } from '../types/events';

function payload(over: Partial<CrashDiagnosis> = {}): CrashDiagnosis {
  return {
    exitCode: 1,
    headline: '第一条结论',
    recognized: true,
    findings: [
      {
        kind: 'mixin',
        role: 'root',
        conclusion: 'c',
        confidence: 'high',
        summary: 's',
        culpritLabel: '问题模组',
        culprits: [],
        evidence: [],
        suggestion: 's',
      },
    ],
    context: [{ label: '实例', value: 'pack' }],
    excerpt: [],
    rawOutput: 'raw',
    reportText: 'report',
    instance: 'pack',
    suggestedJava: null,
    ...over,
  };
}

describe('crashStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('open 展示本次诊断并从折叠状态开始', () => {
    const store = useCrashStore();
    store.toggleRaw();

    store.open(payload());

    expect(store.visible).toBe(true);
    expect(store.diagnosis?.headline).toBe('第一条结论');
    expect(store.showRaw).toBe(false);
  });

  it('第二次 open 整份替换，不留上一次的 findings', () => {
    const store = useCrashStore();
    store.open(payload({ headline: '第一次' }));
    store.open(payload({ headline: '第二次', findings: [] }));

    expect(store.diagnosis?.headline).toBe('第二次');
    expect(store.diagnosis?.findings).toHaveLength(0);
  });

  it('close 只收起，toggleRaw 展开原始输出', () => {
    const store = useCrashStore();
    store.open(payload());

    store.toggleRaw();
    expect(store.showRaw).toBe(true);

    store.close();
    expect(store.visible).toBe(false);
  });
});

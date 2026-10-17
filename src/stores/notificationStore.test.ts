/**
 * 对话框 resolver 槽位测试
 * confirm / choice 各只有一个 resolver 槽位，验证重复调用时前一个 await 不会永久挂起
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { useNotificationStore } from './notificationStore';

describe('notificationStore 对话框', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('第二次 confirm 先把前一个 await 按 false 结算', async () => {
    const store = useNotificationStore();
    const first = store.confirm('t1', 'c1');
    const second = store.confirm('t2', 'c2');

    store.handleConfirm(true);

    await expect(first).resolves.toBe(false);
    await expect(second).resolves.toBe(true);
  });

  it('第二次 choice 先把前一个 await 按 null 结算', async () => {
    const store = useNotificationStore();
    const options = [{ id: 'go', label: '继续' }];
    const first = store.choice('t1', 'c1', options);
    const second = store.choice('t2', 'c2', options);

    store.handleChoice('go');

    await expect(first).resolves.toBeNull();
    await expect(second).resolves.toBe('go');
  });
});

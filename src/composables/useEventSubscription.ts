import { onScopeDispose } from 'vue';
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn, EventCallback } from '@tauri-apps/api/event';

/**
 * 通用的 Tauri 事件订阅 composable
 * 自动处理订阅/取消订阅和作用域销毁时的清理
 */
export function useEventSubscription<T>(
  eventName: string,
  handler: EventCallback<T>
) {
  let unlisten: UnlistenFn | null = null;
  let isSubscribed = false;

  async function subscribe() {
    if (isSubscribed) return;
    
    unlisten = await listen<T>(eventName, handler);
    isSubscribed = true;
  }

  function unsubscribe() {
    if (unlisten) {
      unlisten();
      unlisten = null;
    }
    isSubscribed = false;
  }

  // 作用域销毁时自动清理
  onScopeDispose(unsubscribe);

  return {
    subscribe,
    unsubscribe,
    get isSubscribed() {
      return isSubscribed;
    }
  };
}

/**
 * 多事件订阅管理器
 * 用于同时管理多个事件的订阅
 */
export function useMultiEventSubscription() {
  const subscriptions: Array<{ unsubscribe: () => void }> = [];

  function add<T>(eventName: string, handler: EventCallback<T>) {
    const sub = useEventSubscription(eventName, handler);
    subscriptions.push(sub);
    return sub;
  }

  async function subscribeAll() {
    await Promise.all(subscriptions.map(s => (s as any).subscribe?.()));
  }

  function unsubscribeAll() {
    subscriptions.forEach(s => s.unsubscribe());
  }

  onScopeDispose(unsubscribeAll);

  return {
    add,
    subscribeAll,
    unsubscribeAll
  };
}

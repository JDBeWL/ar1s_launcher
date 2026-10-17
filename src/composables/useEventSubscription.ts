/**
 * Tauri 事件订阅工具
 * 封装 listen 的订阅/退订，作用域销毁时自动退订，支持多事件批量注册
 */
import { onScopeDispose } from 'vue';
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn, EventCallback } from '@tauri-apps/api/event';

export function useEventSubscription<T>(eventName: string, handler: EventCallback<T>) {
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

  onScopeDispose(unsubscribe);

  return {
    subscribe,
    unsubscribe,
    get isSubscribed() {
      return isSubscribed;
    },
  };
}

interface SubscriptionHandle {
  subscribe: () => Promise<void>;
  unsubscribe: () => void;
  isSubscribed: boolean;
}

export function useMultiEventSubscription() {
  const subscriptions: SubscriptionHandle[] = [];

  function add<T>(eventName: string, handler: EventCallback<T>) {
    const sub = useEventSubscription(eventName, handler);
    subscriptions.push(sub);
    return sub;
  }

  async function subscribeAll() {
    await Promise.all(subscriptions.map((s) => s.subscribe()));
  }

  function unsubscribeAll() {
    subscriptions.forEach((s) => s.unsubscribe());
  }

  onScopeDispose(unsubscribeAll);

  return {
    add,
    subscribeAll,
    unsubscribeAll,
  };
}

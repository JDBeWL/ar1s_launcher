import { ref, watch, onScopeDispose } from 'vue';
import type { Ref, WatchSource } from 'vue';

/**
 * 防抖函数 composable
 * @param fn 要防抖的函数
 * @param delay 延迟时间（毫秒）
 */
export function useDebounceFn<T extends (...args: any[]) => any>(
  fn: T,
  delay: number = 300
) {
  let timeoutId: ReturnType<typeof setTimeout> | null = null;

  function debouncedFn(...args: Parameters<T>) {
    if (timeoutId) {
      clearTimeout(timeoutId);
    }
    timeoutId = setTimeout(() => {
      fn(...args);
      timeoutId = null;
    }, delay);
  }

  function cancel() {
    if (timeoutId) {
      clearTimeout(timeoutId);
      timeoutId = null;
    }
  }

  function flush(...args: Parameters<T>) {
    cancel();
    fn(...args);
  }

  // 作用域销毁时清理
  onScopeDispose(cancel);

  return {
    call: debouncedFn,
    cancel,
    flush,
    get isPending() {
      return timeoutId !== null;
    }
  };
}

/**
 * 防抖的响应式值
 * @param source 源响应式值
 * @param delay 延迟时间（毫秒）
 */
export function useDebouncedRef<T>(source: Ref<T>, delay: number = 300): Ref<T> {
  const debouncedValue = ref(source.value) as Ref<T>;
  let timeoutId: ReturnType<typeof setTimeout> | null = null;

  watch(source, (newValue) => {
    if (timeoutId) {
      clearTimeout(timeoutId);
    }
    timeoutId = setTimeout(() => {
      debouncedValue.value = newValue;
      timeoutId = null;
    }, delay);
  });

  onScopeDispose(() => {
    if (timeoutId) {
      clearTimeout(timeoutId);
    }
  });

  return debouncedValue;
}

/**
 * 监听响应式值变化并执行防抖回调
 * @param source 监听源
 * @param callback 回调函数
 * @param delay 延迟时间（毫秒）
 */
export function watchDebounced<T>(
  source: WatchSource<T>,
  callback: (value: T) => void,
  delay: number = 300
) {
  let timeoutId: ReturnType<typeof setTimeout> | null = null;

  const stopWatch = watch(source, (newValue) => {
    if (timeoutId) {
      clearTimeout(timeoutId);
    }
    timeoutId = setTimeout(() => {
      callback(newValue);
      timeoutId = null;
    }, delay);
  });

  function stop() {
    if (timeoutId) {
      clearTimeout(timeoutId);
      timeoutId = null;
    }
    stopWatch();
  }

  onScopeDispose(stop);

  return stop;
}

/**
 * 防抖组合式工具
 * 函数、ref、watcher 三种防抖形态，都在作用域销毁时清理定时器
 */
import { ref, watch, onScopeDispose } from 'vue';
import type { Ref, WatchSource } from 'vue';

export function useDebounceFn<T extends (...args: never[]) => unknown>(fn: T, delay: number = 300) {
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

  onScopeDispose(cancel);

  return {
    call: debouncedFn,
    cancel,
    flush,
    get isPending() {
      return timeoutId !== null;
    },
  };
}

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

export function watchDebounced<T>(source: WatchSource<T>, callback: (value: T) => void, delay: number = 300) {
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

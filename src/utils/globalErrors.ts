/**
 * 全局错误兜底
 * Vue 之外的 window error / unhandledrejection 也接进日志与非侵入提示，重复错误做去重
 */
import type { App } from 'vue';
import { LauncherCommandError } from '../services/invoke';
import { useNotificationStore } from '../stores/notificationStore';
import { getErrorMessage } from './format';
import { logError } from './logger';

export const GLOBAL_ERROR_DEDUPE_MS = 15_000;
export const GLOBAL_ERROR_MESSAGE_MAX_CHARS = 500;

const IGNORED_MESSAGE_PATTERNS = [/ResizeObserver loop/];

interface GlobalErrorReporterOptions {
  notify: (message: string) => void;
  log: (message: string, error: unknown, context: string) => void;
  now?: () => number;
}

function truncate(message: string): string {
  if (message.length <= GLOBAL_ERROR_MESSAGE_MAX_CHARS) return message;
  return `${message.slice(0, GLOBAL_ERROR_MESSAGE_MAX_CHARS)}…`;
}

export function createGlobalErrorReporter(options: GlobalErrorReporterOptions) {
  const now = options.now ?? Date.now;
  const lastSeen = new Map<string, number>();

  function report(source: string, error: unknown, detail?: string): void {
    if (error instanceof LauncherCommandError && error.cancelled) return;

    const message = getErrorMessage(error).trim() || '未知错误';
    if (IGNORED_MESSAGE_PATTERNS.some((pattern) => pattern.test(message))) return;

    const context = detail ? `${source}: ${detail}` : source;
    options.log('界面出现未处理错误', error, context);

    const time = now();
    for (const [signature, timestamp] of lastSeen) {
      if (time - timestamp > GLOBAL_ERROR_DEDUPE_MS * 4) {
        lastSeen.delete(signature);
      }
    }

    const signature = `${source}|${message}`;
    const previous = lastSeen.get(signature);
    if (previous !== undefined && time - previous < GLOBAL_ERROR_DEDUPE_MS) return;

    lastSeen.set(signature, time);
    if (lastSeen.size > 100) {
      const oldest = lastSeen.keys().next().value;
      if (oldest !== undefined) lastSeen.delete(oldest);
    }

    options.notify(truncate(message));
  }

  return { report };
}

const reporter = createGlobalErrorReporter({
  notify: (message) => {
    try {
      useNotificationStore().error('界面出现未处理错误', message);
    } catch {
      // Pinia 还没就绪时只能放弃提示
    }
  },
  log: (message, error, context) => logError(message, error, context),
});

export function reportGlobalError(source: string, error: unknown, detail?: string): void {
  reporter.report(source, error, detail);
}

let installed = false;

export function installGlobalErrorHandlers(app: App): void {
  if (installed) return;
  installed = true;

  app.config.errorHandler = (error, _instance, info) => {
    reportGlobalError('Vue', error, info);
  };

  window.addEventListener('error', (event) => {
    if (!(event instanceof ErrorEvent)) return;
    reportGlobalError('window', event.error ?? event.message, event.filename || undefined);
  });

  window.addEventListener('unhandledrejection', (event) => {
    reportGlobalError('promise', event.reason);
  });
}

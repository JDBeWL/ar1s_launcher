/**
 * 前端错误落盘
 * 生产环境控制台是关的，错误通过 log_frontend_error 写进启动器日志，导出诊断包时才能看到前端上下文
 */
import { invoke } from '@tauri-apps/api/core';

export const MAX_FRONTEND_LOG_MESSAGE_CHARS = 4000;

const DUPLICATE_WINDOW_MS = 5_000;
const RATE_WINDOW_MS = 60_000;
const MAX_REPORTS_PER_WINDOW = 20;

interface FrontendErrorReporterOptions {
  send?: (context: string, message: string) => Promise<unknown>;
  now?: () => number;
  available?: () => boolean;
}

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

function sendFrontendError(context: string, message: string): Promise<unknown> {
  return invoke('log_frontend_error', { context, message });
}

function truncate(message: string): string {
  if (message.length <= MAX_FRONTEND_LOG_MESSAGE_CHARS) return message;
  return `${message.slice(0, MAX_FRONTEND_LOG_MESSAGE_CHARS)}…`;
}

export function createFrontendErrorReporter(options: FrontendErrorReporterOptions = {}) {
  const send = options.send ?? sendFrontendError;
  const now = options.now ?? Date.now;
  const available = options.available ?? isTauriRuntime;
  let windowStartedAt = 0;
  let windowCount = 0;
  const recent = new Map<string, number>();

  function report(context: string, message: string): void {
    if (!available()) return;

    const time = now();
    if (time - windowStartedAt >= RATE_WINDOW_MS) {
      windowStartedAt = time;
      windowCount = 0;
      recent.clear();
    }
    if (windowCount >= MAX_REPORTS_PER_WINDOW) return;

    const signature = `${context}\u0000${message}`;
    const previous = recent.get(signature);
    if (previous !== undefined && time - previous < DUPLICATE_WINDOW_MS) return;

    recent.set(signature, time);
    windowCount += 1;

    try {
      void send(context, truncate(message)).catch(() => {});
    } catch {
      // 日志通道自身失败不能反过来影响业务
    }
  }

  return { report };
}

export const reportFrontendError = createFrontendErrorReporter().report;

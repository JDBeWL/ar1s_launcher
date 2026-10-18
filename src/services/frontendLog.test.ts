import { describe, expect, it, vi } from 'vitest';
import { MAX_FRONTEND_LOG_MESSAGE_CHARS, createFrontendErrorReporter } from './frontendLog';

function setupReporter(options: { available?: () => boolean; now?: () => number } = {}) {
  const send = vi.fn().mockResolvedValue(undefined);
  let now = 0;
  const reporter = createFrontendErrorReporter({
    send,
    available: options.available ?? (() => true),
    now: options.now ?? (() => now),
  });

  return {
    reporter,
    send,
    advance(ms: number) {
      now += ms;
    },
  };
}

describe('frontend error reporter', () => {
  it('相同错误在短时间内只上报一次', () => {
    const { reporter, send, advance } = setupReporter();

    reporter.report('invoke', 'boom');
    reporter.report('invoke', 'boom');
    expect(send).toHaveBeenCalledTimes(1);

    advance(5_000);
    reporter.report('invoke', 'boom');
    expect(send).toHaveBeenCalledTimes(2);
  });

  it('每个窗口最多上报 20 条', () => {
    const { reporter, send } = setupReporter();

    for (let i = 0; i < 25; i += 1) {
      reporter.report('invoke', `error-${i}`);
    }

    expect(send).toHaveBeenCalledTimes(20);
  });

  it('窗口结束后恢复上报', () => {
    const { reporter, send, advance } = setupReporter();

    for (let i = 0; i < 25; i += 1) {
      reporter.report('invoke', `error-${i}`);
    }
    advance(60_000);
    reporter.report('invoke', 'after-window');

    expect(send).toHaveBeenCalledTimes(21);
  });

  it('不在 Tauri 运行时不发命令', () => {
    const { reporter, send } = setupReporter({ available: () => false });

    reporter.report('invoke', 'boom');

    expect(send).not.toHaveBeenCalled();
  });

  it('过长消息会截断', () => {
    const { reporter, send } = setupReporter();

    reporter.report('invoke', 'x'.repeat(MAX_FRONTEND_LOG_MESSAGE_CHARS + 20));

    const message = send.mock.calls[0][1] as string;
    expect(message).toHaveLength(MAX_FRONTEND_LOG_MESSAGE_CHARS + 1);
    expect(message.endsWith('…')).toBe(true);
  });

  it('上报失败不会产生未处理的拒绝', async () => {
    const reporter = createFrontendErrorReporter({
      send: vi.fn().mockRejectedValue(new Error('backend down')),
      available: () => true,
    });

    expect(() => reporter.report('invoke', 'boom')).not.toThrow();
    await Promise.resolve();
  });
});

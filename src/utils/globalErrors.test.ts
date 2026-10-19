import { describe, expect, it, vi } from 'vitest';
import { LauncherCommandError } from '../services/invoke';
import {
  GLOBAL_ERROR_DEDUPE_MS,
  GLOBAL_ERROR_LOG_LABEL,
  GLOBAL_ERROR_MESSAGE_MAX_CHARS,
  createGlobalErrorReporter,
} from './globalErrors';

function setupReporter() {
  const notify = vi.fn();
  const log = vi.fn();
  let now = 0;
  const reporter = createGlobalErrorReporter({
    notify,
    log,
    now: () => now,
  });

  return {
    reporter,
    notify,
    log,
    advance(ms: number) {
      now += ms;
    },
  };
}

describe('global error reporter', () => {
  it('上报错误并提示一次，重复错误只提示一次', () => {
    const { reporter, notify, log } = setupReporter();

    reporter.report('Vue', new Error('boom'));
    reporter.report('Vue', new Error('boom'));

    expect(notify).toHaveBeenCalledTimes(1);
    expect(notify).toHaveBeenCalledWith('boom');
    expect(log).toHaveBeenCalledTimes(2);
  });

  it('去重窗口结束后会再次提示', () => {
    const { reporter, notify, advance } = setupReporter();

    reporter.report('Vue', new Error('boom'));
    advance(GLOBAL_ERROR_DEDUPE_MS);
    reporter.report('Vue', new Error('boom'));

    expect(notify).toHaveBeenCalledTimes(2);
  });

  it('取消类命令错误不打扰用户也不写日志', () => {
    const { reporter, notify, log } = setupReporter();
    const error = new LauncherCommandError('install_modrinth_modpack', 'cancelled', '操作已取消', undefined);

    reporter.report('promise', error);

    expect(notify).not.toHaveBeenCalled();
    expect(log).not.toHaveBeenCalled();
  });

  it('非 Error 载荷也能取出可读消息', () => {
    const { reporter, notify } = setupReporter();

    reporter.report('promise', { message: '后端拒绝' });

    expect(notify).toHaveBeenCalledWith('后端拒绝');
  });

  it('通知内容过长时截断，日志保留完整错误', () => {
    const { reporter, notify, log } = setupReporter();
    const message = 'x'.repeat(GLOBAL_ERROR_MESSAGE_MAX_CHARS + 20);

    reporter.report('window', new Error(message));

    expect((notify.mock.calls[0][0] as string).endsWith('…')).toBe(true);
    expect(log).toHaveBeenCalledWith(GLOBAL_ERROR_LOG_LABEL, expect.any(Error), 'window');
  });

  it('ResizeObserver 循环噪音不提示', () => {
    const { reporter, notify } = setupReporter();

    reporter.report('window', new Error('ResizeObserver loop completed with undelivered notifications'));

    expect(notify).not.toHaveBeenCalled();
  });
});

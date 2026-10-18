import { beforeEach, describe, expect, it, vi } from 'vitest';

const { invokeMock, warnMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  warnMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));
vi.mock('../utils/logger', () => ({ logWarn: warnMock }));

import { DEFAULT_COMMAND_TIMEOUT_MS, LauncherCommandError, invokeCommand } from './invoke';

describe('invokeCommand', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    warnMock.mockReset();
    vi.useRealTimers();
  });

  it('成功时透传结果', async () => {
    invokeMock.mockResolvedValue({ ok: true });

    await expect(invokeCommand('get_versions')).resolves.toEqual({ ok: true });
    expect(invokeMock).toHaveBeenCalledWith('get_versions', undefined);
  });

  it('保留后端稳定错误码与命令名', async () => {
    invokeMock.mockRejectedValue({ code: 'busy', message: '另一个启动器实例正在占用下载锁' });

    const error = await invokeCommand('download_version').catch((value: unknown) => value);

    expect(error).toBeInstanceOf(LauncherCommandError);
    expect(error).toMatchObject({
      command: 'download_version',
      code: 'busy',
      message: '另一个启动器实例正在占用下载锁',
    });
  });

  it('字符串 reject 归为 ipc', async () => {
    invokeMock.mockRejectedValue('invalid args `instanceName` for command');

    await expect(invokeCommand('delete_instance')).rejects.toMatchObject({
      command: 'delete_instance',
      code: 'ipc',
      message: 'invalid args `instanceName` for command',
    });
  });

  it('普通 Error 归为 ipc 并保留消息', async () => {
    invokeMock.mockRejectedValue(new Error('后端未响应'));

    await expect(invokeCommand('get_instances')).rejects.toMatchObject({
      code: 'ipc',
      message: '后端未响应',
    });
  });

  it('cancelled 错误可直接识别', async () => {
    invokeMock.mockRejectedValue({ code: 'cancelled', message: '操作已取消' });

    const error = (await invokeCommand('install_modrinth_modpack').catch(
      (value: unknown) => value,
    )) as LauncherCommandError;

    expect(error.cancelled).toBe(true);
  });

  it('无 message 的对象不会显示成 [object Object]', async () => {
    invokeMock.mockRejectedValue({ code: 'custom', detail: 'x' });

    const error = (await invokeCommand('get_instances').catch((value: unknown) => value)) as LauncherCommandError;

    expect(error.message).not.toBe('[object Object]');
    expect(error.message).toContain('detail');
  });

  it('默认超时后拒绝并标记 timeout', async () => {
    vi.useFakeTimers();
    invokeMock.mockReturnValue(new Promise(() => {}));

    const promise = invokeCommand('get_versions');
    const assertion = expect(promise).rejects.toMatchObject({
      command: 'get_versions',
      code: 'timeout',
    });

    await vi.advanceTimersByTimeAsync(DEFAULT_COMMAND_TIMEOUT_MS);
    await assertion;
  });

  it('下载等长任务不设默认超时', async () => {
    vi.useFakeTimers();
    let resolveRequest: ((value: string) => void) | undefined;
    invokeMock.mockReturnValue(
      new Promise<string>((resolve) => {
        resolveRequest = resolve;
      }),
    );

    let settled = false;
    const promise = invokeCommand('download_version').finally(() => {
      settled = true;
    });

    await vi.advanceTimersByTimeAsync(DEFAULT_COMMAND_TIMEOUT_MS * 5);
    expect(settled).toBe(false);

    resolveRequest?.('done');
    await expect(promise).resolves.toBe('done');
  });

  it('可显式关闭单次调用的超时', async () => {
    vi.useFakeTimers();
    let resolveRequest: ((value: string) => void) | undefined;
    invokeMock.mockReturnValue(
      new Promise<string>((resolve) => {
        resolveRequest = resolve;
      }),
    );

    let settled = false;
    const promise = invokeCommand('get_versions', undefined, { timeoutMs: 0 }).finally(() => {
      settled = true;
    });

    await vi.advanceTimersByTimeAsync(DEFAULT_COMMAND_TIMEOUT_MS * 2);
    expect(settled).toBe(false);

    resolveRequest?.('done');
    await expect(promise).resolves.toBe('done');
  });
});

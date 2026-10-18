import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

const { saveMock, exportMock } = vi.hoisted(() => ({
  saveMock: vi.fn(),
  exportMock: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({ save: saveMock }));
vi.mock('../services', () => ({ instanceApi: { exportDiagnostics: exportMock } }));
vi.mock('../utils/logger', () => ({ logError: vi.fn() }));

import { useDiagnosticsExport } from './useDiagnosticsExport';
import { useNotificationStore } from '../stores/notificationStore';
import type { GameInstance } from '../types/events';

const instance: GameInstance = {
  id: '1',
  name: 'Vanilla Perfected',
  version: '26.3',
  path: 'D:/mc/versions/Vanilla Perfected',
};

describe('useDiagnosticsExport', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    saveMock.mockReset();
    exportMock.mockReset();
  });

  it('取消保存时不调用后端', async () => {
    saveMock.mockResolvedValue(null);
    const { exportDiagnostics } = useDiagnosticsExport();

    await exportDiagnostics(instance);

    expect(exportMock).not.toHaveBeenCalled();
  });

  it('自动补 .zip 并提示导出路径', async () => {
    saveMock.mockResolvedValue('D:/diag/ar1s-diagnostics');
    exportMock.mockResolvedValue(undefined);
    const notifications = useNotificationStore();
    const { exportDiagnostics } = useDiagnosticsExport();

    await exportDiagnostics(instance);

    expect(exportMock).toHaveBeenCalledWith('Vanilla Perfected', 'D:/diag/ar1s-diagnostics.zip');
    expect(notifications.notifications[notifications.notifications.length - 1]).toMatchObject({
      type: 'success',
      title: '诊断包已导出',
    });
  });

  it('失败时给出错误通知并释放导出状态', async () => {
    saveMock.mockResolvedValue('D:/diag/out.zip');
    exportMock.mockRejectedValue(new Error('disk full'));
    const notifications = useNotificationStore();
    const { exportDiagnostics, exportingInstance } = useDiagnosticsExport();

    await exportDiagnostics(instance);

    expect(notifications.notifications[notifications.notifications.length - 1]).toMatchObject({
      type: 'error',
      title: '导出诊断包失败',
    });
    expect(exportingInstance.value).toBeNull();
  });

  it('导出进行中忽略重复点击', async () => {
    let resolveSave!: (value: string | null) => void;
    saveMock.mockReturnValue(
      new Promise<string | null>((resolve) => {
        resolveSave = resolve;
      }),
    );
    const { exportDiagnostics } = useDiagnosticsExport();

    const first = exportDiagnostics(instance);
    await exportDiagnostics(instance);
    expect(saveMock).toHaveBeenCalledTimes(1);

    resolveSave(null);
    await first;
  });
});

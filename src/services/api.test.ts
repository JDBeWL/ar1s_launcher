import { beforeEach, describe, expect, it, vi } from 'vitest';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

import { CACHE_MISS, CacheKeys, cache } from './cache';
import { instanceApi, modpackApi, versionApi } from './api';

const gameDirInfoKey = `${CacheKeys.CONFIG}:game_dir_info`;

function seedGameDirInfoCache() {
  cache.set(gameDirInfoKey, { path: 'D:/mc', versions: ['1.20.1'] });
}

describe('game dir info cache invalidation', () => {
  beforeEach(() => {
    cache.clear();
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(undefined);
  });

  it.each([
    ['创建实例', () => instanceApi.createInstance('new-instance', '1.20.1')],
    ['删除实例', () => instanceApi.deleteInstance('old-instance')],
    ['重命名实例', () => instanceApi.renameInstance('old-instance', 'new-instance')],
    ['下载版本', () => versionApi.downloadVersion('1.20.1')],
    [
      '安装整合包',
      () =>
        modpackApi.installModrinthModpack({
          modpack_id: 'project',
          version_id: 'version',
          instance_name: 'pack',
          install_path: 'D:/mc',
        }),
    ],
  ])('%s后失效已安装版本缓存', async (_label, run) => {
    seedGameDirInfoCache();

    await run();

    expect(cache.get(gameDirInfoKey)).toBe(CACHE_MISS);
  });
});

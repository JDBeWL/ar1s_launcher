/**
 * 简单的内存缓存，用于缓存 API 响应
 */

interface CacheEntry<T> {
  data: T;
  timestamp: number;
  expiresAt: number;
}

class MemoryCache {
  private cache = new Map<string, CacheEntry<any>>();
  private defaultTTL = 5 * 60 * 1000; // 默认 5 分钟

  /**
   * 获取缓存
   */
  get<T>(key: string): T | null {
    const entry = this.cache.get(key);
    
    if (!entry) {
      return null;
    }

    // 检查是否过期
    if (Date.now() > entry.expiresAt) {
      this.cache.delete(key);
      return null;
    }

    return entry.data as T;
  }

  /**
   * 设置缓存
   * @param key 缓存键
   * @param data 数据
   * @param ttl 过期时间（毫秒），默认 5 分钟
   */
  set<T>(key: string, data: T, ttl?: number): void {
    const now = Date.now();
    this.cache.set(key, {
      data,
      timestamp: now,
      expiresAt: now + (ttl ?? this.defaultTTL),
    });
  }

  /**
   * 删除缓存
   */
  delete(key: string): boolean {
    return this.cache.delete(key);
  }

  /**
   * 删除匹配前缀的所有缓存
   */
  deleteByPrefix(prefix: string): number {
    let count = 0;
    for (const key of this.cache.keys()) {
      if (key.startsWith(prefix)) {
        this.cache.delete(key);
        count++;
      }
    }
    return count;
  }

  /**
   * 清除所有缓存
   */
  clear(): void {
    this.cache.clear();
  }

  /**
   * 清除过期缓存
   */
  cleanup(): number {
    const now = Date.now();
    let count = 0;
    
    for (const [key, entry] of this.cache.entries()) {
      if (now > entry.expiresAt) {
        this.cache.delete(key);
        count++;
      }
    }
    
    return count;
  }

  /**
   * 获取缓存大小
   */
  get size(): number {
    return this.cache.size;
  }
}

// 全局缓存实例
export const cache = new MemoryCache();

// 缓存键前缀常量
export const CacheKeys = {
  VERSIONS: 'versions',
  INSTANCES: 'instances',
  JAVA: 'java',
  LOADERS: 'loaders',
  CONFIG: 'config',
} as const;

/**
 * 带缓存的异步函数包装器
 */
export async function withCache<T>(
  key: string,
  fetcher: () => Promise<T>,
  ttl?: number
): Promise<T> {
  // 先检查缓存
  const cached = cache.get<T>(key);
  if (cached !== null) {
    return cached;
  }

  // 获取数据
  const data = await fetcher();
  
  // 存入缓存
  cache.set(key, data, ttl);
  
  return data;
}

/**
 * 使缓存失效的装饰器
 * 用于写操作后清除相关缓存
 */
export function invalidateCache(...prefixes: string[]) {
  return function <T extends (...args: any[]) => Promise<any>>(fn: T): T {
    return (async (...args: any[]) => {
      const result = await fn(...args);
      
      // 清除相关缓存
      for (const prefix of prefixes) {
        cache.deleteByPrefix(prefix);
      }
      
      return result;
    }) as T;
  };
}

// 定期清理过期缓存（每 5 分钟）
if (typeof window !== 'undefined') {
  setInterval(() => {
    const cleaned = cache.cleanup();
    if (cleaned > 0) {
      console.debug(`[Cache] Cleaned ${cleaned} expired entries`);
    }
  }, 5 * 60 * 1000);
}

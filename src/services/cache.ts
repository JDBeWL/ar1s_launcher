/**
 * 简单的内存缓存，用于缓存 API 响应
 */
import { logDebug } from '../utils/logger';

interface CacheEntry<T> {
  data: T;
  timestamp: number;
  expiresAt: number;
}

const CACHE_MISS_SYMBOL = Symbol('CACHE_MISS');

class MemoryCache {
  private cache = new Map<string, CacheEntry<unknown>>();
  private defaultTTL = 5 * 60 * 1000;
  private maxSize = 200;

  has(key: string): boolean {
    const entry = this.cache.get(key);
    if (!entry) return false;
    if (Date.now() > entry.expiresAt) {
      this.cache.delete(key);
      return false;
    }
    return true;
  }

  private evictOldest(): void {
    let oldestKey: string | null = null;
    let oldestTime = Infinity;
    for (const [key, entry] of this.cache.entries()) {
      if (entry.timestamp < oldestTime) {
        oldestTime = entry.timestamp;
        oldestKey = key;
      }
    }
    if (oldestKey !== null) {
      this.cache.delete(oldestKey);
    }
  }

  get<T>(key: string): T | typeof CACHE_MISS_SYMBOL {
    const entry = this.cache.get(key);
    
    if (!entry) {
      return CACHE_MISS_SYMBOL;
    }

    if (Date.now() > entry.expiresAt) {
      this.cache.delete(key);
      return CACHE_MISS_SYMBOL;
    }

    entry.timestamp = Date.now();

    return entry.data as T;
  }

  /**
   * 设置缓存
   * @param key 缓存键
   * @param data 数据
   * @param ttl 过期时间（毫秒），默认 5 分钟
   */
  set<T>(key: string, data: T, ttl?: number): void {
    if (this.cache.size >= this.maxSize) {
      this.evictOldest();
    }
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
  const cached = cache.get<T>(key);
  if (cached !== CACHE_MISS_SYMBOL) {
    return cached;
  }

  const data = await fetcher();
  cache.set(key, data, ttl);
  
  return data;
}

// 定期清理过期缓存（每 5 分钟）
if (typeof window !== 'undefined') {
  setInterval(() => {
      const cleaned = cache.cleanup();
      if (cleaned > 0) {
        logDebug(`Cleaned ${cleaned} expired entries`, 'Cache');
      }
  }, 5 * 60 * 1000);
}

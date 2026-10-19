/**
 * 实例列表的进程内缓存
 *
 * 列表页和实例详情页之间来回切换时，组件会重新挂载。没有缓存的话每次都要先铺骨架屏
 * 再换成真实列表，视觉上会闪一下；这里保留上一次的结果，让新实例先渲染旧数据再静默刷新。
 */
import type { GameInstance } from '../types/events';

let cache: GameInstance[] = [];

export function getInstanceListCache(): GameInstance[] {
  return cache;
}

export function setInstanceListCache(instances: GameInstance[]): void {
  // 存副本，避免把响应式代理带进下一次挂载
  cache = [...instances];
}

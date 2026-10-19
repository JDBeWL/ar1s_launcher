/**
 * 实例列表的进程内缓存
 * 列表页与详情页来回切换时组件会重新挂载；保留上一次结果，先渲染旧数据再静默刷新，避免闪骨架屏。
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

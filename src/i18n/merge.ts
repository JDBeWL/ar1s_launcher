/**
 * 多语言消息合并
 *
 * 每个命名空间单独一个片段文件，按语言目录聚合；同名命名空间深合并，
 * 这样多个片段可以各自贡献一部分键而不互相覆盖。
 */

/** 消息树：叶子是字符串，中间节点是同样结构的对象 */
export interface MessageTree {
  [key: string]: string | MessageTree;
}

function isPlainObject(value: unknown): value is MessageTree {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function mergeInto(target: MessageTree, source: MessageTree): void {
  for (const [key, value] of Object.entries(source)) {
    const existing = target[key];
    if (isPlainObject(existing) && isPlainObject(value)) {
      mergeInto(existing, value);
    } else {
      target[key] = value;
    }
  }
}

export function mergeMessages(...sources: MessageTree[]): MessageTree {
  const target: MessageTree = {};
  for (const source of sources) {
    mergeInto(target, source);
  }
  return target;
}

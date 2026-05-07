/**
 * 格式化时间戳为相对时间（如"3 小时前"）
 */
export function formatTimeAgo(timestamp: number): string {
  const diff = Date.now() - timestamp
  const minutes = Math.floor(diff / 60000)
  const hours = Math.floor(diff / 3600000)
  const days = Math.floor(diff / 86400000)
  
  if (minutes < 1) return '刚刚'
  if (minutes < 60) return `${minutes} 分钟前`
  if (hours < 24) return `${hours} 小时前`
  if (days < 7) return `${days} 天前`
  if (days < 30) return `${Math.floor(days / 7)} 周前`
  return new Date(timestamp).toLocaleDateString()
}

/**
 * 格式化上次游玩时间
 */
export function formatLastPlayed(timestamp?: number): string {
  if (!timestamp) return '从未启动'
  return formatTimeAgo(timestamp)
}

/** 图标配置 */
const LOADER_ICONS: Record<string, { list: string; select: string }> = {
  forge: { list: 'mdi-anvil', select: 'mdi-anvil' },
  fabric: { list: 'mdi-texture-box', select: 'mdi-feather' },
  quilt: { list: 'mdi-quilt', select: 'mdi-square-rounded' },
  neoforge: { list: 'mdi-anvil', select: 'mdi-anvil' },
  modded: { list: 'mdi-puzzle', select: 'mdi-puzzle' },
  unknown: { list: 'mdi-help-circle', select: 'mdi-help-circle' },
  none: { list: 'mdi-minecraft', select: 'mdi-close-circle-outline' },
}

/** 加载器主题色配置 */
const LOADER_COLORS: Record<string, { color: string; bgColor: string }> = {
  forge: { color: 'blue', bgColor: 'blue-container' },
  fabric: { color: 'red', bgColor: 'red-container' },
  quilt: { color: 'purple', bgColor: 'purple-container' },
  neoforge: { color: 'orange', bgColor: 'orange-container' },
  modded: { color: 'teal', bgColor: 'teal-container' },
  unknown: { color: 'grey', bgColor: 'grey-container' },
  none: { color: 'primary', bgColor: 'primary-container' },
}

/**
 * 根据加载器类型获取对应主题色
 */
export function getLoaderColor(loaderType?: string): { color: string; bgColor: string } {
  const key = (loaderType || 'none').toLowerCase()
  return LOADER_COLORS[key] || LOADER_COLORS['none']
}

/**
 * 根据加载器类型获取对应图标
 * @param loaderType 加载器类型
 * @param context 使用场景：'list' 用于实例列表，'select' 用于选择界面
 */
export function getLoaderIcon(loaderType?: string, context: 'list' | 'select' = 'list'): string {
  const key = (loaderType || 'none').toLowerCase()
  const icons = LOADER_ICONS[key]
  
  if (icons) {
    return context === 'select' ? icons.select : icons.list
  }
  
  return context === 'select' ? 'mdi-puzzle' : 'mdi-minecraft'
}

/**
 * 从错误对象中提取错误消息
 * 处理多种错误格式：Error 对象、Tauri 错误 {message: "..."} 和字符串
 */
export function getErrorMessage(error: unknown): string {
  if (error instanceof Error) return error.message
  if (typeof error === 'string') return error
  if (error && typeof error === 'object' && 'message' in error) {
    return String((error as { message: unknown }).message)
  }
  return String(error)
}

interface ParsedVersion {
  type: 'release' | 'rc' | 'pre' | 'snapshot' | 'other' | 'unknown'
  parts: number[]
  suffixNum: number
}

const VERSION_TYPE_PRIORITY: Record<string, number> = {
  release: 3,
  rc: 2,
  pre: 1,
  other: 0,
  snapshot: -1,
  unknown: -2,
}

function parseVersion(v: string): ParsedVersion {
  const snapshotMatch = v.match(/^(\d+)w(\d+)([a-z])$/)
  if (snapshotMatch) {
    return { type: 'snapshot', parts: [parseInt(snapshotMatch[1]), parseInt(snapshotMatch[2])], suffixNum: 0 }
  }

  const match = v.match(/^([\d.]+)(?:-(.+))?$/)
  if (!match) return { type: 'unknown', parts: [0], suffixNum: 0 }

  const parts = match[1].split('.').map(n => parseInt(n) || 0)
  const suffix = match[2] || ''

  if (suffix.startsWith('rc')) {
    return { type: 'rc', parts, suffixNum: parseInt(suffix.slice(2)) || 0 }
  }
  if (suffix.startsWith('pre')) {
    return { type: 'pre', parts, suffixNum: parseInt(suffix.slice(3)) || 0 }
  }
  if (suffix) {
    return { type: 'other', parts, suffixNum: 0 }
  }

  return { type: 'release', parts, suffixNum: 0 }
}

const parseVersionCache = new Map<string, ParsedVersion>()

function cachedParseVersion(v: string): ParsedVersion {
  let parsed = parseVersionCache.get(v)
  if (!parsed) {
    parsed = parseVersion(v)
    parseVersionCache.set(v, parsed)
  }
  return parsed
}

export function compareVersionDesc(a: string, b: string): number {
  const va = cachedParseVersion(a)
  const vb = cachedParseVersion(b)

  if (va.type === 'snapshot' && vb.type !== 'snapshot') return 1
  if (vb.type === 'snapshot' && va.type !== 'snapshot') return -1

  const maxLen = Math.max(va.parts.length, vb.parts.length)
  for (let i = 0; i < maxLen; i++) {
    const av = va.parts[i] ?? 0
    const bv = vb.parts[i] ?? 0
    if (av !== bv) return bv - av
  }

  const typeDiff = (VERSION_TYPE_PRIORITY[vb.type] ?? 0) - (VERSION_TYPE_PRIORITY[va.type] ?? 0)
  if (typeDiff !== 0) return typeDiff

  return (vb.suffixNum ?? 0) - (va.suffixNum ?? 0)
}

export type SortOrder = 'newest' | 'oldest';

export interface VersionLike {
  id?: string;
  version?: string;
  releaseTime?: string;
}

function getReleaseTime(item: VersionLike): number {
  if (!item.releaseTime) return 0;
  return new Date(item.releaseTime).getTime() || 0;
}

export function sortVersionsByReleaseTime<T extends VersionLike>(
  items: T[],
  order: SortOrder = 'newest'
): T[] {
  return [...items].sort((a, b) => {
    const timeA = getReleaseTime(a);
    const timeB = getReleaseTime(b);
    return order === 'newest' ? timeB - timeA : timeA - timeB;
  });
}

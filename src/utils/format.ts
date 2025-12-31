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
  none: { list: 'mdi-minecraft', select: 'mdi-close-circle-outline' },
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
 * @deprecated 使用 getLoaderIcon(loader, 'select') 代替
 */
export function getLoaderSelectIcon(loader: string): string {
  return getLoaderIcon(loader, 'select')
}

/**
 * 从错误对象中提取错误消息
 */
export function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

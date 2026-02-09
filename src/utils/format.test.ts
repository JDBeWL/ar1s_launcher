import { describe, it, expect, vi, afterEach } from 'vitest'
import { formatTimeAgo, formatLastPlayed, getLoaderIcon, getErrorMessage } from './format'

describe('formatTimeAgo', () => {
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('刚刚（不到1分钟）', () => {
    const now = Date.now()
    expect(formatTimeAgo(now)).toBe('刚刚')
    expect(formatTimeAgo(now - 30_000)).toBe('刚刚') // 30秒前
  })

  it('N 分钟前', () => {
    const now = Date.now()
    expect(formatTimeAgo(now - 60_000)).toBe('1 分钟前')
    expect(formatTimeAgo(now - 30 * 60_000)).toBe('30 分钟前')
    expect(formatTimeAgo(now - 59 * 60_000)).toBe('59 分钟前')
  })

  it('N 小时前', () => {
    const now = Date.now()
    expect(formatTimeAgo(now - 3_600_000)).toBe('1 小时前')
    expect(formatTimeAgo(now - 12 * 3_600_000)).toBe('12 小时前')
    expect(formatTimeAgo(now - 23 * 3_600_000)).toBe('23 小时前')
  })

  it('N 天前', () => {
    const now = Date.now()
    expect(formatTimeAgo(now - 86_400_000)).toBe('1 天前')
    expect(formatTimeAgo(now - 6 * 86_400_000)).toBe('6 天前')
  })

  it('N 周前', () => {
    const now = Date.now()
    expect(formatTimeAgo(now - 7 * 86_400_000)).toBe('1 周前')
    expect(formatTimeAgo(now - 14 * 86_400_000)).toBe('2 周前')
    expect(formatTimeAgo(now - 29 * 86_400_000)).toBe('4 周前')
  })

  it('超过30天显示具体日期', () => {
    const now = Date.now()
    const result = formatTimeAgo(now - 31 * 86_400_000)
    // 应该返回 locale 日期字符串，不再是 "X 周前"
    expect(result).not.toContain('周前')
    expect(result).not.toContain('天前')
  })
})

describe('formatLastPlayed', () => {
  it('无时间戳返回"从未启动"', () => {
    expect(formatLastPlayed()).toBe('从未启动')
    expect(formatLastPlayed(undefined)).toBe('从未启动')
    expect(formatLastPlayed(0)).toBe('从未启动')
  })

  it('有时间戳时委托给 formatTimeAgo', () => {
    const now = Date.now()
    expect(formatLastPlayed(now)).toBe('刚刚')
    expect(formatLastPlayed(now - 3_600_000)).toBe('1 小时前')
  })
})

describe('getLoaderIcon', () => {
  // 列表场景
  it('Forge 列表图标', () => {
    expect(getLoaderIcon('forge', 'list')).toBe('mdi-anvil')
    expect(getLoaderIcon('Forge', 'list')).toBe('mdi-anvil')
  })

  it('Fabric 列表图标', () => {
    expect(getLoaderIcon('fabric', 'list')).toBe('mdi-texture-box')
  })

  it('Quilt 列表图标', () => {
    expect(getLoaderIcon('quilt', 'list')).toBe('mdi-quilt')
  })

  it('NeoForge 列表图标', () => {
    expect(getLoaderIcon('neoforge', 'list')).toBe('mdi-anvil')
  })

  it('None / 原版列表图标', () => {
    expect(getLoaderIcon('None', 'list')).toBe('mdi-minecraft')
    expect(getLoaderIcon('none', 'list')).toBe('mdi-minecraft')
  })

  // 选择场景
  it('Fabric 选择图标', () => {
    expect(getLoaderIcon('fabric', 'select')).toBe('mdi-feather')
  })

  it('None 选择图标', () => {
    expect(getLoaderIcon('none', 'select')).toBe('mdi-close-circle-outline')
  })

  // 默认值
  it('undefined 默认为 list 的 none', () => {
    expect(getLoaderIcon(undefined)).toBe('mdi-minecraft')
  })

  it('未知加载器 list 返回 mdi-minecraft', () => {
    expect(getLoaderIcon('unknown', 'list')).toBe('mdi-minecraft')
  })

  it('未知加载器 select 返回 mdi-puzzle', () => {
    expect(getLoaderIcon('unknown', 'select')).toBe('mdi-puzzle')
  })
})

describe('getErrorMessage', () => {
  it('Error 对象提取 message', () => {
    expect(getErrorMessage(new Error('test error'))).toBe('test error')
  })

  it('字符串直接返回', () => {
    expect(getErrorMessage('string error')).toBe('string error')
  })

  it('数字转为字符串', () => {
    expect(getErrorMessage(404)).toBe('404')
  })

  it('null 转为字符串', () => {
    expect(getErrorMessage(null)).toBe('null')
  })

  it('undefined 转为字符串', () => {
    expect(getErrorMessage(undefined)).toBe('undefined')
  })

  it('对象转为字符串', () => {
    expect(getErrorMessage({ code: 500 })).toBe('[object Object]')
  })
})


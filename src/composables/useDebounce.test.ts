import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { ref, nextTick } from 'vue'
import { useDebounceFn, useDebouncedRef, watchDebounced } from './useDebounce'

describe('useDebounceFn', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('防抖函数延迟执行', async () => {
    const fn = vi.fn()
    const { call } = useDebounceFn(fn, 300)

    call()
    expect(fn).not.toHaveBeenCalled()

    vi.advanceTimersByTime(300)
    expect(fn).toHaveBeenCalledTimes(1)
  })

  it('快速多次调用只执行最后一次', async () => {
    const fn = vi.fn()
    const { call } = useDebounceFn(fn, 300)

    call(1)
    call(2)
    call(3)

    vi.advanceTimersByTime(300)
    expect(fn).toHaveBeenCalledTimes(1)
    expect(fn).toHaveBeenCalledWith(3)
  })

  it('cancel 取消执行', () => {
    const fn = vi.fn()
    const { call, cancel } = useDebounceFn(fn, 300)

    call()
    cancel()

    vi.advanceTimersByTime(300)
    expect(fn).not.toHaveBeenCalled()
  })

  it('flush 立即执行', () => {
    const fn = vi.fn()
    const { call, flush } = useDebounceFn(fn, 300)

    call(1)
    flush(2)

    expect(fn).toHaveBeenCalledTimes(1)
    expect(fn).toHaveBeenCalledWith(2)

    vi.advanceTimersByTime(300)
    expect(fn).toHaveBeenCalledTimes(1) // 不会再次执行
  })

  it('isPending 反映状态', () => {
    const fn = vi.fn()
    const debounced = useDebounceFn(fn, 300)

    expect(debounced.isPending).toBe(false)

    debounced.call()
    expect(debounced.isPending).toBe(true)

    vi.advanceTimersByTime(300)
    expect(debounced.isPending).toBe(false)
  })
})

describe('useDebouncedRef', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('延迟更新值', async () => {
    const source = ref('initial')
    const debounced = useDebouncedRef(source, 300)

    expect(debounced.value).toBe('initial')

    source.value = 'updated'
    await nextTick()
    expect(debounced.value).toBe('initial') // 还没更新

    vi.advanceTimersByTime(300)
    await nextTick()
    expect(debounced.value).toBe('updated')
  })

  it('快速多次变化只更新最后一次', async () => {
    const source = ref('a')
    const debounced = useDebouncedRef(source, 300)

    source.value = 'b'
    source.value = 'c'
    source.value = 'd'
    await nextTick()

    vi.advanceTimersByTime(300)
    await nextTick()
    expect(debounced.value).toBe('d')
  })
})

describe('watchDebounced', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('防抖监听值变化', async () => {
    const source = ref(0)
    const callback = vi.fn()
    const stop = watchDebounced(source, callback, 300)

    source.value = 1
    await nextTick()
    expect(callback).not.toHaveBeenCalled()

    vi.advanceTimersByTime(300)
    await nextTick()
    expect(callback).toHaveBeenCalledTimes(1)
    expect(callback).toHaveBeenCalledWith(1)

    stop()
  })

  it('stop 停止监听', async () => {
    const source = ref(0)
    const callback = vi.fn()
    const stop = watchDebounced(source, callback, 300)

    source.value = 1
    await nextTick()
    stop()

    vi.advanceTimersByTime(300)
    await nextTick()
    expect(callback).not.toHaveBeenCalled()
  })
})


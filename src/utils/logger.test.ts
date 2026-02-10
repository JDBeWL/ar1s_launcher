import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { logger, logError, logWarn, logInfo, logDebug } from './logger'

describe('Logger', () => {
  let consoleSpy: {
    debug: ReturnType<typeof vi.spyOn>
    info: ReturnType<typeof vi.spyOn>
    warn: ReturnType<typeof vi.spyOn>
    error: ReturnType<typeof vi.spyOn>
  }

  beforeEach(() => {
    consoleSpy = {
      debug: vi.spyOn(console, 'debug').mockImplementation(() => {}),
      info: vi.spyOn(console, 'info').mockImplementation(() => {}),
      warn: vi.spyOn(console, 'warn').mockImplementation(() => {}),
      error: vi.spyOn(console, 'error').mockImplementation(() => {}),
    }
  })

  afterEach(() => {
    Object.values(consoleSpy).forEach(spy => spy.mockRestore())
  })

  describe('logger methods', () => {
    it('logger.debug 调用 console.debug', () => {
      logger.debug('test message', 'TestContext')
      expect(consoleSpy.debug).toHaveBeenCalledWith('[TestContext] test message', '')
    })

    it('logger.info 调用 console.info', () => {
      logger.info('test message', 'TestContext')
      expect(consoleSpy.info).toHaveBeenCalledWith('[TestContext] test message', '')
    })

    it('logger.warn 调用 console.warn', () => {
      const error = new Error('test error')
      logger.warn('test message', error, 'TestContext')
      expect(consoleSpy.warn).toHaveBeenCalledWith('[TestContext] test message', error)
    })

    it('logger.error 调用 console.error 并输出 stack', () => {
      const error = new Error('test error')
      logger.error('test message', error, 'TestContext')
      expect(consoleSpy.error).toHaveBeenCalledWith('[TestContext] test message', error)
      expect(consoleSpy.error).toHaveBeenCalledWith('Stack:', expect.any(String))
    })
  })

  describe('便捷函数', () => {
    it('logDebug 调用 logger.debug', () => {
      logDebug('test', 'Context')
      expect(consoleSpy.debug).toHaveBeenCalled()
    })

    it('logInfo 调用 logger.info', () => {
      logInfo('test', 'Context')
      expect(consoleSpy.info).toHaveBeenCalled()
    })

    it('logWarn 调用 logger.warn', () => {
      logWarn('test', undefined, 'Context')
      expect(consoleSpy.warn).toHaveBeenCalled()
    })

    it('logError 调用 logger.error', () => {
      logError('test', undefined, 'Context')
      expect(consoleSpy.error).toHaveBeenCalled()
    })
  })

  describe('上下文处理', () => {
    it('无上下文时不添加前缀', () => {
      logger.info('test message')
      expect(consoleSpy.info).toHaveBeenCalledWith('test message', '')
    })

    it('有上下文时添加前缀', () => {
      logger.info('test message', 'MyComponent')
      expect(consoleSpy.info).toHaveBeenCalledWith('[MyComponent] test message', '')
    })
  })
})


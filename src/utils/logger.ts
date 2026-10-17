/**
 * 前端日志模块
 * 带 [context] 前缀包装 console，生产环境完全静默以免泄露敏感信息
 */
type LogLevel = 'debug' | 'info' | 'warn' | 'error';

class Logger {
  private formatMessage(_level: LogLevel, message: string, context?: string): string {
    const prefix = context ? `[${context}]` : '';
    return `${prefix} ${message}`.trim();
  }

  private log(level: LogLevel, message: string, error?: unknown, context?: string): void {
    const formattedMessage = this.formatMessage(level, message, context);

    if (import.meta.env.DEV) {
      switch (level) {
        case 'debug':
          console.debug(formattedMessage, error || '');
          break;
        case 'info':
          console.info(formattedMessage, error || '');
          break;
        case 'warn':
          console.warn(formattedMessage, error || '');
          break;
        case 'error':
          console.error(formattedMessage, error || '');
          if (error instanceof Error) {
            console.error('Stack:', error.stack);
          }
          break;
      }
    } else {
      // 生产环境：禁用控制台输出，避免泄露敏感信息
    }
  }

  debug(message: string, context?: string): void {
    this.log('debug', message, undefined, context);
  }

  info(message: string, context?: string): void {
    this.log('info', message, undefined, context);
  }

  warn(message: string, error?: unknown, context?: string): void {
    this.log('warn', message, error, context);
  }

  error(message: string, error?: unknown, context?: string): void {
    this.log('error', message, error, context);
  }
}

export const logger = new Logger();

export const logError = (message: string, error?: unknown, context?: string) => {
  logger.error(message, error, context);
};

export const logWarn = (message: string, error?: unknown, context?: string) => {
  logger.warn(message, error, context);
};

export const logInfo = (message: string, context?: string) => {
  logger.info(message, context);
};

export const logDebug = (message: string, context?: string) => {
  logger.debug(message, context);
};

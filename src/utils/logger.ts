/**
 * 前端日志模块
 * 带 [context] 前缀包装 console，生产环境完全静默以免泄露敏感信息
 */
import { reportFrontendError } from '../services/frontendLog';

type LogLevel = 'debug' | 'info' | 'warn' | 'error';

class Logger {
  private formatMessage(_level: LogLevel, message: string, context?: string): string {
    const prefix = context ? `[${context}]` : '';
    return `${prefix} ${message}`.trim();
  }

  private log(level: LogLevel, message: string, error?: unknown, context?: string): void {
    const formattedMessage = this.formatMessage(level, message, context);

    if (level === 'error') {
      const detail = error instanceof Error ? error.stack || error.message : error == null ? '' : String(error);
      reportFrontendError(context ?? 'Frontend', detail ? `${formattedMessage}\n${detail}` : formattedMessage);
    }

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
      // 生产环境不打 console，错误已通过 log_frontend_error 写进启动器日志
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

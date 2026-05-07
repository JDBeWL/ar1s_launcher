/**
 * 统一日志系统
 * 提供统一的日志接口，便于后续扩展（如发送到后端、日志文件等）
 */

type LogLevel = 'debug' | 'info' | 'warn' | 'error';

class Logger {
  private formatMessage(_level: LogLevel, message: string, context?: string): string {
    const prefix = context ? `[${context}]` : '';
    return `${prefix} ${message}`.trim();
  }

  private log(level: LogLevel, message: string, error?: unknown, context?: string): void {
    const formattedMessage = this.formatMessage(level, message, context);

    // 根据环境决定输出方式
    if (import.meta.env.DEV) {
      // 开发环境：详细输出
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
      // 未来可以添加：通过 Tauri invoke 将错误发送到后端日志系统
      // if (level === 'error') {
      //   invoke('log_to_file', { level: 'error', message: formattedMessage });
      // }
    }

    // 未来可以在这里添加：发送到后端、写入日志文件等
    // if (level === 'error') {
    //   this.sendToBackend(entry);
    // }
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

// 导出单例
export const logger = new Logger();

// 便捷函数
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


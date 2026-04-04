export interface LogEntry {
  timestamp: string;
  level: "info" | "warn" | "error";
  message: string;
  device?: string;
}

export function createLogEntry(
  message: string,
  level: LogEntry["level"] = "info"
): LogEntry {
  return {
    timestamp: new Date().toISOString(),
    level,
    message,
  };
}

export function formatLogEntry(entry: LogEntry): string {
  const time = new Date(entry.timestamp).toLocaleTimeString();
  const prefix = {
    info: "ℹ️",
    warn: "⚠️",
    error: "❌",
  }[entry.level];
  
  return `[${time}] ${prefix} ${entry.message}`;
}

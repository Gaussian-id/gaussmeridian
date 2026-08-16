/**
 * * Logger Function
 *
 * @param context - The context of the log
 * @param data - The data to log
 * @param type - The type of log
 * @returns void
 */
function getReadableUTCTimestamp() {
  const now = new Date();
  const year = now.getUTCFullYear();
  const month = String(now.getUTCMonth() + 1).padStart(2, "0");
  const day = String(now.getUTCDate()).padStart(2, "0");
  const hour = String(now.getUTCHours()).padStart(2, "0");
  const minute = String(now.getUTCMinutes()).padStart(2, "0");
  const second = String(now.getUTCSeconds()).padStart(2, "0");
  const ms = String(now.getUTCMilliseconds()).padStart(3, "0");

  return `${year}-${month}-${day} - ${hour}:${minute}:${second}.${ms} UTC`;
}

export function logger(
  context: string,
  data: unknown,
  type: "info" | "warn" | "error" | "dir" | "table" | "json" | "group" | "groupEnd" = "info",
): void {
  const timestamp = getReadableUTCTimestamp();

  switch (type) {
    case "info":
      console.info(`[${timestamp}] [INFO] [${context}]`, data);
      break;
    case "warn":
      console.warn(`[${timestamp}] [WARN] [${context}]`, data);
      break;
    case "error":
      console.error(`[${timestamp}] [ERROR] [${context}]`, data);
      break;
    case "dir":
      console.log(`[${timestamp}] [DIR] [${context}]`);
      console.dir(data, { depth: null, colors: true });
      break;
    case "table":
      console.log(`[${timestamp}] [TABLE] [${context}]`);
      console.table(data);
      break;
    case "json":
      console.log(`[${timestamp}] [JSON] [${context}]`);
      console.log(JSON.stringify(data, null, 2));
      break;
    case "group":
      console.group(`[${timestamp}] [GROUP] [${context}]`);
      break;
    case "groupEnd":
      console.log(`[${timestamp}] [GROUP END] [${context}]`);
      console.groupEnd();
      break;
    default:
      console.info(`[${timestamp}] [INFO] [${context}]`, data);
  }
}

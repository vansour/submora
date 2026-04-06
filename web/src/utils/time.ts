export function formatOptionalTimestamp(timestamp: number | null): string {
  if (timestamp === null) {
    return "未记录";
  }

  const date = new Date(timestamp * 1000);
  if (Number.isNaN(date.getTime())) {
    return "未记录";
  }

  return date.toISOString();
}

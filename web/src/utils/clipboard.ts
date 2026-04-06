export function buildPublicRouteUrl(username: string, origin = currentOrigin()): string {
  return `${origin}/${username}`;
}

export async function copyPublicRoute(username: string): Promise<string> {
  const url = buildPublicRouteUrl(username);

  if (typeof navigator === "undefined" || navigator.clipboard === undefined) {
    return url;
  }

  try {
    await navigator.clipboard.writeText(url);
    return url;
  } catch (error) {
    throw extractClipboardError(error);
  }
}

function currentOrigin(): string {
  if (typeof window === "undefined") {
    return "";
  }

  return window.location.origin;
}

function extractClipboardError(error: unknown): string {
  if (error instanceof Error && error.message.trim() !== "") {
    return error.message;
  }

  if (typeof error === "string" && error.trim() !== "") {
    return error;
  }

  return "复制公共入口链接失败";
}

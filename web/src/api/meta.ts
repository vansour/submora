import { requestWithoutBody } from "@/api/client";
import type { AppInfoResponse } from "@/api/types";

export function getAppInfo(): Promise<AppInfoResponse> {
  return requestWithoutBody<AppInfoResponse>("GET", "/api/meta/app");
}

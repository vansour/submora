import {
  clearCsrfTokenCache,
  requestCurrentUser,
  requestJsonWithBody,
  requestWithoutBody,
} from "@/api/client";
import type {
  ApiErrorBody,
  ApiMessage,
  CurrentUserResponse,
  LoginRequest,
  UpdateAccountRequest,
} from "@/api/types";
import { translateBackendMessage } from "@/utils/messages";

export async function getCurrentUser(): Promise<CurrentUserResponse | null> {
  const response = await requestCurrentUser();

  if (response.status === 401) {
    clearCsrfTokenCache();
    return null;
  }

  if (!response.ok) {
    let message = `Request failed with status ${response.status}`;

    try {
      const body = (await response.json()) as ApiErrorBody;
      if (typeof body.message === "string" && body.message.trim() !== "") {
        message = translateBackendMessage(body.message);
      }
    } catch {
      // Ignore JSON parse failures and keep the fallback message.
    }

    throw message;
  }

  return (await response.json()) as CurrentUserResponse;
}

export async function login(payload: LoginRequest): Promise<ApiMessage> {
  const response = await requestJsonWithBody<ApiMessage>("POST", "/api/auth/login", payload);
  clearCsrfTokenCache();
  return response;
}

export async function logout(): Promise<ApiMessage> {
  const response = await requestWithoutBody<ApiMessage>("POST", "/api/auth/logout");
  clearCsrfTokenCache();
  return response;
}

export async function updateAccount(payload: UpdateAccountRequest): Promise<ApiMessage> {
  const response = await requestJsonWithBody<ApiMessage>("PUT", "/api/auth/account", payload);
  clearCsrfTokenCache();
  return response;
}

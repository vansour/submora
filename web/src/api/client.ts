import type { ApiErrorBody, CsrfTokenResponse } from "@/api/types";
import { translateBackendMessage } from "@/utils/messages";

const CSRF_HEADER = "x-csrf-token";

let csrfTokenCache: string | null = null;

export function clearCsrfTokenCache(): void {
  csrfTokenCache = null;
}

function defaultErrorMessage(status: number): string {
  return `Request failed with status ${status}`;
}

async function parseError(response: Response): Promise<string> {
  const fallback = defaultErrorMessage(response.status);

  try {
    const body = (await response.json()) as ApiErrorBody;
    if (typeof body.message === "string" && body.message.trim() !== "") {
      return translateBackendMessage(body.message);
    }
  } catch {
    return fallback;
  }

  return fallback;
}

async function parseJson<T>(response: Response): Promise<T> {
  if (!response.ok) {
    throw await parseError(response);
  }

  return (await response.json()) as T;
}

async function fetchJson<T>(input: RequestInfo | URL, init?: RequestInit): Promise<T> {
  const response = await fetch(input, init);
  return parseJson<T>(response);
}

async function fetchCsrfToken(): Promise<string> {
  const payload = await fetchJson<CsrfTokenResponse>("/api/auth/csrf", {
    credentials: "include",
  });
  csrfTokenCache = payload.token;
  return payload.token;
}

async function csrfToken(): Promise<string> {
  if (csrfTokenCache !== null) {
    return csrfTokenCache;
  }

  return fetchCsrfToken();
}

async function send(input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
  return fetch(input, {
    credentials: "include",
    ...init,
  });
}

async function sendWithCsrf(input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
  const token = await csrfToken();
  const headers = new Headers(init?.headers);
  headers.set(CSRF_HEADER, token);

  return send(input, {
    ...init,
    headers,
  });
}

function methodAllowsAnonymousSend(method: string): boolean {
  return method === "GET" || method === "HEAD";
}

function jsonRequestInit(method: string, body: unknown, headers?: HeadersInit): RequestInit {
  const nextHeaders = new Headers(headers);
  nextHeaders.set("content-type", "application/json");

  return {
    method,
    headers: nextHeaders,
    body: JSON.stringify(body),
  };
}

export async function requestJson<T>(input: RequestInfo | URL, init?: RequestInit): Promise<T> {
  const method = (init?.method ?? "GET").toUpperCase();

  if (methodAllowsAnonymousSend(method)) {
    return fetchJson<T>(input, {
      credentials: "include",
      ...init,
      method,
    });
  }

  const sendStateChangingRequest = async (): Promise<Response> =>
    sendWithCsrf(input, {
      ...init,
      method,
    });

  let response = await sendStateChangingRequest();
  if (response.status === 403) {
    clearCsrfTokenCache();
    response = await sendStateChangingRequest();
  }

  return parseJson<T>(response);
}

export async function requestJsonWithBody<T>(
  method: "POST" | "PUT" | "PATCH",
  input: RequestInfo | URL,
  body: unknown,
): Promise<T> {
  return requestJson<T>(input, jsonRequestInit(method, body));
}

export async function requestWithoutBody<T>(
  method: "GET" | "HEAD" | "POST" | "DELETE",
  input: RequestInfo | URL,
): Promise<T> {
  return requestJson<T>(input, { method });
}

export async function requestCurrentUser(): Promise<Response> {
  return send("/api/auth/me");
}

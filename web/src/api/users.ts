import {
  requestJsonWithBody,
  requestWithoutBody,
} from "@/api/client";
import type {
  ApiMessage,
  CreateUserRequest,
  LinksPayload,
  UserLinksResponse,
  UserOrderPayload,
  UserSummary,
} from "@/api/types";

export function listUsers(): Promise<UserSummary[]> {
  return requestWithoutBody<UserSummary[]>("GET", "/api/users");
}

export function createUser(payload: CreateUserRequest): Promise<UserSummary> {
  return requestJsonWithBody<UserSummary>("POST", "/api/users", payload);
}

export function deleteUser(username: string): Promise<ApiMessage> {
  return requestWithoutBody<ApiMessage>("DELETE", `/api/users/${username}`);
}

export function getLinks(username: string): Promise<UserLinksResponse> {
  return requestWithoutBody<UserLinksResponse>("GET", `/api/users/${username}/links`);
}

export function setLinks(username: string, payload: LinksPayload): Promise<UserLinksResponse> {
  return requestJsonWithBody<UserLinksResponse>(
    "PUT",
    `/api/users/${username}/links`,
    payload,
  );
}

export function setOrder(payload: UserOrderPayload): Promise<string[]> {
  return requestJsonWithBody<string[]>("PUT", "/api/users/order", payload);
}

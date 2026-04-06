export interface ApiMessage {
  message: string;
}

export interface ApiErrorBody {
  error: string;
  message: string;
}

export interface AppInfoResponse {
  name: string;
  phase: number;
  frontend: string;
  backend: string;
  version: string;
  web_dist_dir: string;
}

export interface LoginRequest {
  username: string;
  password: string;
}

export interface CurrentUserResponse {
  username: string;
}

export interface CsrfTokenResponse {
  token: string;
}

export interface UserSummary {
  username: string;
}

export interface CreateUserRequest {
  username: string;
}

export interface LinksPayload {
  links: string[];
}

export interface UserLinksResponse {
  username: string;
  links: string[];
}

export interface UserOrderPayload {
  order: string[];
}

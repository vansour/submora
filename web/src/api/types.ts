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

export interface UpdateAccountRequest {
  current_password: string | null;
  new_username: string;
  new_password: string;
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

export interface LinkDiagnostic {
  url: string;
  status: string;
  detail: string | null;
  http_status: number | null;
  content_type: string | null;
  body_bytes: number | null;
  redirect_count: number;
  is_html: boolean;
  fetched_at: number | null;
}

export interface UserDiagnosticsResponse {
  username: string;
  diagnostics: LinkDiagnostic[];
}

export interface UserCacheStatusResponse {
  username: string;
  state: string;
  line_count: number;
  body_bytes: number;
  generated_at: number | null;
  expires_at: number | null;
}

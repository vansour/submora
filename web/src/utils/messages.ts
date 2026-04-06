const FIELD_LABELS: Array<[prefix: string, label: string]> = [
  ["username: ", "用户名："],
  ["password: ", "密码："],
  ["new_username: ", "新用户名："],
  ["new_password: ", "新密码："],
  ["current_password: ", "当前密码："],
  ["links: ", "链接："],
  ["order: ", "排序："],
];

export function translateBackendMessage(message: string): string {
  for (const [prefix, label] of FIELD_LABELS) {
    if (message.startsWith(prefix)) {
      return `${label}${translateDetail(message.slice(prefix.length))}`;
    }
  }

  return translateDetail(message);
}

export function extractFieldValidationError(
  message: string,
  fieldName: string,
  localizedFieldName: string,
): string | null {
  const prefixes = [
    `${fieldName}: `,
    `${localizedFieldName}：`,
    `${localizedFieldName}: `,
  ];

  for (const prefix of prefixes) {
    if (message.startsWith(prefix)) {
      return message.slice(prefix.length).trim();
    }
  }

  return null;
}

function translateDetail(detail: string): string {
  switch (detail) {
    case "Please login":
      return "请先登录";
    case "invalid username":
      return "用户名不合法";
    case "password must be 1-128 characters":
      return "密码长度必须为 1 到 128 个字符";
    case "password must include letters, numbers, and symbols":
      return "密码必须同时包含字母、数字和符号";
    case "current password is required":
      return "必须填写当前密码";
    case "current password is incorrect":
      return "当前密码不正确";
    case "change username or enter a new password":
      return "请至少修改用户名或填写新密码";
    case "username already exists":
      return "用户名已存在";
    case "must not be empty":
      return "不能为空";
    case "user not found":
      return "订阅组不存在";
    case "missing csrf token in session":
      return "会话中缺少 CSRF 令牌";
    case "missing csrf token header":
      return "请求缺少 CSRF 令牌";
    case "invalid csrf token":
      return "CSRF 令牌无效";
    case "Fetch completed successfully":
      return "抓取成功";
    case "No fetch attempt recorded yet":
      return "尚未记录抓取尝试";
    case "order must include every existing user exactly once":
      return "排序必须且只能包含每个现有用户一次";
    default:
      break;
  }

  const loginRetry = parseRetryMessage(detail, "too many login attempts, retry in ");
  if (loginRetry !== null) {
    return `登录尝试过多，请在 ${loginRetry} 秒后重试`;
  }

  const publicRetry = parseRetryMessage(detail, "too many public requests, retry in ");
  if (publicRetry !== null) {
    return `公共请求过多，请在 ${publicRetry} 秒后重试`;
  }

  const maxUsers = parseMaximumMessage(detail, " users allowed");
  if (maxUsers !== null) {
    return `最多允许 ${maxUsers} 个订阅组`;
  }

  const maxAllowed = parseMaximumMessage(detail, " allowed");
  if (maxAllowed !== null) {
    return `最多允许 ${maxAllowed} 项`;
  }

  if (detail.startsWith("invalid username: ")) {
    return `用户名不合法：${detail.slice("invalid username: ".length)}`;
  }
  if (detail.startsWith("duplicate username: ")) {
    return `用户名重复：${detail.slice("duplicate username: ".length)}`;
  }
  if (detail.startsWith("invalid url: ")) {
    return `无效的 URL：${detail.slice("invalid url: ".length)}`;
  }
  if (detail.startsWith("unsupported scheme: ")) {
    return `不支持的协议：${detail.slice("unsupported scheme: ".length)}`;
  }
  if (detail.startsWith("missing host: ")) {
    return `缺少主机名：${detail.slice("missing host: ".length)}`;
  }
  if (detail.startsWith("failed to resolve host: ")) {
    return `解析主机失败：${detail.slice("failed to resolve host: ".length)}`;
  }
  if (detail.startsWith("unsafe target: ")) {
    return `不安全的目标：${detail.slice("unsafe target: ".length)}`;
  }
  if (detail.startsWith("redirect missing location header: ")) {
    return `重定向响应缺少 Location 头：${detail.slice("redirect missing location header: ".length)}`;
  }
  if (detail.startsWith("redirect location is not valid utf-8: ")) {
    return `重定向 Location 不是有效的 UTF-8：${detail.slice("redirect location is not valid utf-8: ".length)}`;
  }
  if (detail.startsWith("invalid redirect target from ")) {
    return `重定向目标无效：${detail.slice("invalid redirect target from ".length)}`;
  }
  if (detail.startsWith("content too large: exceeds ")) {
    return `内容过大：流式读取时超过 ${detail.slice("content too large: exceeds ".length)}`;
  }
  if (detail.startsWith("failed to fetch ")) {
    return `抓取失败：${detail.slice("failed to fetch ".length)}`;
  }
  if (detail.startsWith("too many redirects while fetching ")) {
    return `抓取时重定向过多：${detail.slice("too many redirects while fetching ".length)}`;
  }
  if (detail.startsWith("unexpected response status ")) {
    return `收到异常响应状态：${detail.slice("unexpected response status ".length)}`;
  }
  if (detail.startsWith("content too large while fetching ")) {
    return `抓取内容过大：${detail.slice("content too large while fetching ".length)}`;
  }

  return detail;
}

function parseRetryMessage(detail: string, prefix: string): string | null {
  if (!detail.startsWith(prefix) || !detail.endsWith("s")) {
    return null;
  }

  return detail.slice(prefix.length, -1);
}

function parseMaximumMessage(detail: string, suffix: string): string | null {
  const prefix = "maximum ";
  if (!detail.startsWith(prefix) || !detail.endsWith(suffix)) {
    return null;
  }

  return detail.slice(prefix.length, -suffix.length);
}

import {
  extractFieldValidationError,
  translateBackendMessage,
} from "@/utils/messages";

describe("message translation", () => {
  it("translates prefixed validation messages", () => {
    expect(translateBackendMessage("links: invalid url: ftp://bad.example/feed")).toBe(
      "链接：无效的 URL：ftp://bad.example/feed",
    );
    expect(translateBackendMessage("password: password must be 1-128 characters")).toBe(
      "密码：密码长度必须为 1 到 128 个字符",
    );
  });

  it("extracts field-level messages from localized and raw prefixes", () => {
    expect(extractFieldValidationError("links: invalid url: bad", "links", "链接")).toBe(
      "invalid url: bad",
    );
    expect(extractFieldValidationError("链接：无效的 URL：bad", "links", "链接")).toBe(
      "无效的 URL：bad",
    );
    expect(extractFieldValidationError("username already exists", "links", "链接")).toBeNull();
  });
});

import { expect, test, type Page } from "@playwright/test";

const alphaUrl = "http://example.test:19081/feed";

test.setTimeout(60_000);

async function closeToasts(page: Page): Promise<void> {
  const buttons = page.getByTestId("toast-close");
  while ((await buttons.count()) > 0) {
    await buttons.first().click();
  }
}

test("admin console flow works end-to-end", async ({ page }) => {
  await page.goto("/login");

  await expect(page.getByTestId("login-username")).toBeEnabled();
  await page.getByTestId("login-username").fill("admin");
  await page.getByTestId("login-password").fill("admin");
  await page.getByTestId("login-submit").click();

  await expect(page).toHaveURL(/\/console$/);
  await expect(page.getByText("admin")).toBeVisible();

  await page.reload();
  await expect(page).toHaveURL(/\/console$/);
  await expect(page.getByText("admin")).toBeVisible();

  await page.getByTestId("create-user-input").fill("alpha");
  await page.getByTestId("create-user-submit").click();
  await expect(page).toHaveURL(/\/users\/alpha$/);

  await page.getByTestId("create-user-input").fill("beta");
  await page.getByTestId("create-user-submit").click();
  await expect(page).toHaveURL(/\/users\/beta$/);

  await page.getByTestId("user-item-beta-move-up").click();
  const firstUser = page.locator('[data-testid^="user-item-"]').first();
  await expect(firstUser).toHaveAttribute("data-testid", "user-item-beta");

  await page.getByTestId("user-item-alpha-select").click();
  await expect(page).toHaveURL(/\/users\/alpha$/);

  await page.getByTestId("link-row-input-0").fill(alphaUrl);
  await page.getByTestId("editor-save").click();
  await expect(page.getByText("已保存 alpha 的源链接，共 1 条")).toBeVisible();
  await closeToasts(page);

  await page.getByTestId("topbar-open-account").click();
  await page.getByTestId("account-new-username").fill("admin_stage12");
  await page.getByTestId("account-current-password").fill("admin");
  await page.getByTestId("account-submit").click();

  await expect(page).toHaveURL(/\/login$/);
  await expect(page.getByText("账户已更新，请重新登录")).toBeVisible();
  await closeToasts(page);

  await expect(page.getByTestId("login-username")).toBeEnabled();
  await page.getByTestId("login-username").fill("admin_stage12");
  await page.getByTestId("login-password").fill("admin");
  await page.getByTestId("login-submit").click();
  await expect(page).toHaveURL(/\/console$/);
  await expect(page.getByText("admin_stage12")).toBeVisible();

  page.once("dialog", (dialog) => dialog.accept());
  await page.getByTestId("user-item-beta-delete").click();
  await expect(page.getByTestId("user-item-beta")).toHaveCount(0);

  await page.getByTestId("user-item-alpha-select").click();
  page.once("dialog", (dialog) => dialog.accept());
  await page.getByTestId("user-item-alpha-delete").click();
  await expect(page.getByTestId("user-item-alpha")).toHaveCount(0);
  await expect(page.getByText("还没有订阅组，先创建一个入口。")).toBeVisible();
});

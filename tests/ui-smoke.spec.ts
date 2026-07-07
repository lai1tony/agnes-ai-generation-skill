import { expect, test } from "@playwright/test";

test("main workspaces render and switch", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Agnes AI Studio" })).toBeVisible();
  const nav = page.locator(".nav");

  for (const label of ["文本", "图片", "视频", "历史", "设置"]) {
    await nav.getByRole("button", { name: label, exact: true }).click();
    await expect(nav.getByRole("button", { name: label, exact: true })).toHaveClass(/active/);
  }

  await nav.getByRole("button", { name: "视频", exact: true }).click();
  await expect(page.getByLabel("分辨率")).toHaveValue("720p");
  await expect(page.getByLabel("宽高比")).toHaveValue("16:9");
  await expect(page.getByLabel("视频长度")).toHaveValue("5");
  await expect(page.getByLabel("宽度")).toHaveCount(0);
  await expect(page.getByLabel("帧数")).toHaveCount(0);

  await nav.getByRole("button", { name: "设置", exact: true }).click();
  await expect(page.getByText("API Key", { exact: true })).toBeVisible();
  await page.screenshot({ path: "test-results/ui-smoke-desktop.png", fullPage: true });
});

test("browser preview shows a clear Tauri backend message", async ({ page }) => {
  await page.goto("/");
  await page.locator(".nav").getByRole("button", { name: "设置", exact: true }).click();
  await page.getByLabel("Agnes API Key").fill("test-key");
  await page.getByRole("button", { name: "保存" }).click();

  await expect(page.getByRole("heading", { name: "错误详情" })).toBeVisible();
  await expect(page.getByText("当前页面运行在浏览器预览中")).toBeVisible();
  await expect(page.getByText("{}")).toHaveCount(0);
});

import { expect, test, type Page } from '@playwright/test';

async function backgroundColor(page: Page, selector: string): Promise<string> {
  return page.locator(selector).evaluate((element) => getComputedStyle(element).backgroundColor);
}

async function hasHorizontalOverflow(page: Page): Promise<boolean> {
  return page.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth);
}

test('chat surface follows the dark workspace visual contract and completes a fixture turn', async ({ page }) => {
  await page.goto('/e2e/chat-page-fixture.html');

  await expect(page.getByRole('main', { name: 'Chat da sessão' })).toBeVisible();
  expect(await backgroundColor(page, '.chat-page')).toBe('rgb(17, 25, 37)');
  expect(await backgroundColor(page, '.chat-composer textarea')).toBe('rgb(14, 20, 29)');

  await page.getByRole('textbox', { name: 'Mensagem' }).fill('Olá, Hank');
  await page.getByRole('button', { name: 'Enviar mensagem' }).click();
  await expect(page.getByText('Resposta da fixture.')).toBeVisible();
  await expect(page.getByRole('status')).toHaveText('Concluída');
});

test('chat surface stays within the viewport on a narrow screen', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/e2e/chat-page-fixture.html');

  await expect(page.getByRole('main', { name: 'Chat da sessão' })).toBeVisible();
  expect(await hasHorizontalOverflow(page)).toBe(false);
  await expect(page.getByRole('textbox', { name: 'Mensagem' })).toBeVisible();
});

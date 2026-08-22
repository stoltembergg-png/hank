import { expect, test } from '@playwright/test';

test('desktop frontend renders the project workspace without a Tauri bridge failure', async ({ page }) => {
  await page.goto('/');

  await expect(page.getByRole('heading', { name: 'Hank Desktop' })).toBeVisible();
  await expect(page.getByRole('region', { name: 'Gerenciamento de Projetos' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Abrir formulário de criação de projeto' })).toBeVisible();

  await page.getByRole('button', { name: 'Abrir formulário de criação de projeto' }).click();
  await expect(page.getByRole('form', { name: 'Criar novo projeto' })).toBeVisible();
});

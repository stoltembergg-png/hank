export type McpServerInput = { name: string; endpoint: string; capabilities: string[] };
export type RevokeScope = { server_id: string; project_id: string };
const ALLOWED_CAPABILITIES = new Set(['read', 'cancel']);

export function validateMcpServer(input: McpServerInput): { ok: true } | { ok: false; error: string } {
  let url: URL;
  try { url = new URL(input.endpoint); } catch { return { ok: false, error: 'endpoint inválido' }; }
  if (url.protocol !== 'https:') return { ok: false, error: 'HTTPS obrigatório' };
  if (url.username || url.password) return { ok: false, error: 'credenciais na URL não são permitidas' };
  if (!input.name.trim() || input.name.length > 128) return { ok: false, error: 'nome inválido' };
  if (input.capabilities.some((capability) => !ALLOWED_CAPABILITIES.has(capability))) return { ok: false, error: 'capability não autorizada' };
  return { ok: true };
}

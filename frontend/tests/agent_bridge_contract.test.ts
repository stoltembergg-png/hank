import { describe, expect, it } from 'vitest';
import { AgentBridgeUnavailableError, DesktopAgentApiClient } from '@/api/agents';

describe('Desktop agent bridge contract', () => {
  it('fails closed when the Tauri IPC bridge is unavailable', async () => {
    await expect(
      new DesktopAgentApiClient().list({ project_id: 'prj_1', limit: 10, offset: 0 }),
    ).rejects.toBeInstanceOf(AgentBridgeUnavailableError);

    await expect(
      new DesktopAgentApiClient().list({ project_id: 'prj_1', limit: 10, offset: 0 }),
    ).rejects.toMatchObject({ code: 'AGENT_BRIDGE_UNAVAILABLE' });
  });
});

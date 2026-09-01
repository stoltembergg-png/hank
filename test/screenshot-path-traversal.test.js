import assert from 'node:assert/strict';
import test from 'node:test';
import fs from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';

/**
 * Security test suite for path traversal vulnerability mitigation in screenshot functionality.
 * 
 * This test validates the fix applied to desktop-e2e/specs/project-lifecycle.e2e.mjs
 * where the screenshot method was vulnerable to path traversal attacks.
 * 
 * The mitigation uses path.resolve() and path.relative() to ensure that the target
 * file path stays within the intended diagnostics directory.
 * 
 * Vulnerability: An attacker could use path traversal sequences (../) to write files
 * outside the intended diagnostics directory, potentially overwriting sensitive files.
 * 
 * Mitigation: The fix validates that the resolved target path is within the base
 * directory by checking if the relative path starts with '..' or is absolute.
 */

// Mock WebDriverSession class with only the screenshot method for testing
class WebDriverSessionMock {
  constructor(sessionId, diagnosticsDir) {
    this.sessionId = sessionId;
    this.diagnostics = diagnosticsDir;
  }

  async request(method, route) {
    // Mock screenshot response - return a simple base64 encoded string
    if (method === 'GET' && route.includes('/screenshot')) {
      // Return a minimal valid PNG base64 (1x1 transparent pixel)
      return 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==';
    }
    throw new Error('Unexpected request');
  }

  // This is the patched screenshot method with path traversal protection
  async screenshot(name) {
    const value = await this.request('GET', `/session/${this.sessionId}/screenshot`);
    const base = path.resolve(this.diagnostics);
    const target = path.resolve(base, `${name}.png`);
    const relative = path.relative(base, target);
    if (relative.startsWith('..') || path.isAbsolute(relative)) {
      throw new Error('Invalid file path');
    }
    await fs.writeFile(target, Buffer.from(value, 'base64'));
  }
}

test('screenshot rejects path traversal with parent directory references', async () => {
  const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), 'screenshot-test-'));
  try {
    const session = new WebDriverSessionMock('test-session', tmpDir);
    
    // Attempt path traversal using ../
    await assert.rejects(
      async () => await session.screenshot('../etc/passwd'),
      { message: 'Invalid file path' },
      'Should reject path traversal with ../'
    );

    // Verify no file was created outside the directory
    const parentDir = path.dirname(tmpDir);
    const traversedPath = path.join(parentDir, 'etc', 'passwd.png');
    await assert.rejects(
      async () => await fs.access(traversedPath),
      'File should not exist outside diagnostics directory'
    );
  } finally {
    await fs.rm(tmpDir, { recursive: true, force: true });
  }
});

test('screenshot rejects path traversal with multiple parent directory references', async () => {
  const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), 'screenshot-test-'));
  try {
    const session = new WebDriverSessionMock('test-session', tmpDir);
    
    // Attempt path traversal using ../../
    await assert.rejects(
      async () => await session.screenshot('../../sensitive-file'),
      { message: 'Invalid file path' },
      'Should reject path traversal with ../../'
    );
  } finally {
    await fs.rm(tmpDir, { recursive: true, force: true });
  }
});

test('screenshot rejects absolute paths', async () => {
  const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), 'screenshot-test-'));
  try {
    const session = new WebDriverSessionMock('test-session', tmpDir);
    
    // Attempt to write to absolute path
    const absolutePath = path.join(os.tmpdir(), 'malicious-screenshot');
    await assert.rejects(
      async () => await session.screenshot(absolutePath),
      { message: 'Invalid file path' },
      'Should reject absolute paths'
    );

    // Verify file was not created at absolute path
    await assert.rejects(
      async () => await fs.access(`${absolutePath}.png`),
      'File should not exist at absolute path'
    );
  } finally {
    await fs.rm(tmpDir, { recursive: true, force: true });
  }
});

test('screenshot allows valid filenames within the diagnostics directory', async () => {
  const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), 'screenshot-test-'));
  try {
    const session = new WebDriverSessionMock('test-session', tmpDir);
    
    // Valid screenshot name
    await session.screenshot('valid-screenshot');
    
    // Verify file was created in the correct location
    const expectedPath = path.join(tmpDir, 'valid-screenshot.png');
    await fs.access(expectedPath);
    
    // Verify file content is correct
    const content = await fs.readFile(expectedPath);
    assert.ok(content.length > 0, 'Screenshot file should have content');
  } finally {
    await fs.rm(tmpDir, { recursive: true, force: true });
  }
});

test('screenshot allows valid filenames with hyphens and numbers', async () => {
  const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), 'screenshot-test-'));
  try {
    const session = new WebDriverSessionMock('test-session', tmpDir);
    
    // Valid screenshot names with various characters (from actual e2e test)
    const validNames = ['test-123', 'failure-startup', '01-created', 'after-restart-1'];
    
    for (const name of validNames) {
      await session.screenshot(name);
      const expectedPath = path.join(tmpDir, `${name}.png`);
      await fs.access(expectedPath);
    }
  } finally {
    await fs.rm(tmpDir, { recursive: true, force: true });
  }
});

test('screenshot path validation uses resolved paths to prevent bypass', async () => {
  const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), 'screenshot-test-'));
  try {
    const session = new WebDriverSessionMock('test-session', tmpDir);
    
    // Attempt to bypass with current directory reference followed by parent
    await assert.rejects(
      async () => await session.screenshot('./subdir/../../../etc/passwd'),
      { message: 'Invalid file path' },
      'Should reject complex path traversal attempts'
    );
  } finally {
    await fs.rm(tmpDir, { recursive: true, force: true });
  }
});

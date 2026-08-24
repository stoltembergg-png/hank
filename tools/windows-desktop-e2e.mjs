import { readFile } from 'node:fs/promises';

const timeoutMs = 30_000;
const executableArgument = process.argv.indexOf('--executable');
const portArgument = process.argv.indexOf('--port');
const pidArgument = process.argv.indexOf('--pid');
const executablePath = executableArgument >= 0 ? process.argv[executableArgument + 1] : '';
const port = portArgument >= 0 ? Number(process.argv[portArgument + 1]) : 0;
const processId = pidArgument >= 0 ? Number(process.argv[pidArgument + 1]) : undefined;

if (!executablePath || !Number.isInteger(port) || port <= 0) {
  throw new Error('usage: node tools/windows-desktop-e2e.mjs --executable <path> --port <port> [--pid <pid>]');
}

function readPeSubsystem(bytes) {
  const peOffset = bytes.readUInt32LE(0x3c);
  const optionalHeader = peOffset + 4 + 20;
  const magic = bytes.readUInt16LE(optionalHeader);
  if (magic !== 0x10b && magic !== 0x20b) {
    throw new Error(`unsupported PE optional header: 0x${magic.toString(16)}`);
  }
  return bytes.readUInt16LE(optionalHeader + 68);
}

async function readPage() {
  const response = await fetch(`http://127.0.0.1:${port}/json`, {
    signal: AbortSignal.timeout(1_000),
  });
  if (!response.ok) {
    throw new Error(`DevTools returned HTTP ${response.status}`);
  }
  const targets = await response.json();
  return targets.find((target) => target.type === 'page' && target.title === 'Hank Desktop') ?? null;
}

const executable = await readFile(executablePath);
const subsystem = readPeSubsystem(executable);
if (subsystem !== 2) {
  throw new Error(`desktop release must use IMAGE_SUBSYSTEM_WINDOWS_GUI (2), found ${subsystem}`);
}

const deadline = Date.now() + timeoutMs;
let page = null;
while (Date.now() < deadline) {
  try {
    page = await readPage();
    if (page) break;
  } catch {
    // WebView2 may expose the DevTools endpoint only after the native host.
  }
  await new Promise((resolve) => setTimeout(resolve, 250));
}

if (!page) {
  throw new Error(`desktop WebView did not expose a DevTools page on port ${port}`);
}
if (page.title !== 'Hank Desktop') {
  throw new Error(`desktop loaded unexpected web content: '${page.title}'`);
}
if (!page.url.startsWith('http://tauri.localhost/')) {
  throw new Error(`desktop loaded unexpected page URL: '${page.url}'`);
}

console.log(
  JSON.stringify({
    status: 'passed',
    executable: executablePath,
    processId,
    browserTitle: page.title,
    browserUrl: page.url,
    contentVerified: true,
  }),
);

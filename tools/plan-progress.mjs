import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { basename, resolve } from 'node:path';

export const PROGRESS_BRANCH = 'automation/plan-progress';
export const PROGRESS_START = '<!-- HANK_PLAN_PROGRESS:START -->';
export const PROGRESS_END = '<!-- HANK_PLAN_PROGRESS:END -->';

const CARD_HEADING = /^###\s+PR-(\d{3,})(?:\s+—.*)?\s*$/gm;

function cardLabel(card) {
  return `PR-${String(card).padStart(3, '0')}`;
}

function sortedUniqueCards(cards) {
  const values = cards.map((card) => Number(card));
  if (values.some((card) => !Number.isInteger(card) || card < 1)) {
    throw new Error('plan cards must be positive integers');
  }
  const unique = new Set();
  for (const card of values) {
    if (unique.has(card)) throw new Error(`duplicate plan card ${cardLabel(card)}`);
    unique.add(card);
  }
  return [...unique].sort((left, right) => left - right);
}

export function extractPlannedCards(queueTexts) {
  if (!Array.isArray(queueTexts)) throw new TypeError('queueTexts must be an array');

  const cards = [];
  for (const text of queueTexts) {
    if (typeof text !== 'string') throw new TypeError('queue text must be a string');
    for (const match of text.matchAll(CARD_HEADING)) cards.push(Number(match[1]));
  }
  if (cards.length === 0) throw new Error('no plan cards found in queue');
  return sortedUniqueCards(cards);
}

function flattenRecords(value) {
  if (!Array.isArray(value)) return [value];
  return value.flatMap((item) => flattenRecords(item));
}

function metadataPlanCard(item) {
  const direct = item.planCard ?? item.plan_card;
  if (direct !== undefined && direct !== null) {
    if (Number.isInteger(Number(direct)) && Number(direct) > 0) {
      return { value: Number(direct), declared: true };
    }
    const directMatch = String(direct).match(/PR-(\d{3,})/i);
    if (directMatch) return { value: Number(directMatch[1]), declared: true };
    return { value: null, declared: true };
  }

  const metadata = [item.title, item.body].filter((value) => typeof value === 'string').join('\n');
  const match = metadata.match(/(?:^|\r?\n)\s*(?:plan[ -]card|card do plano)\s*:\s*PR-(\d{3,})\b/i);
  if (match) return { value: Number(match[1]), declared: true };

  const noCard = metadata.match(
    /(?:^|\r?\n)\s*(?:plan[ -]card|card do plano)\s*:\s*(?:none|n\/a|not applicable|fora do plano)\s*(?:\r?\n|$)/i,
  );
  return { value: null, declared: Boolean(noCard) };
}

export function parseMergedPullRequests(value) {
  const records = new Map();
  for (const item of flattenRecords(value)) {
    if (!item || typeof item !== 'object') continue;
    const number = Number(item.number);
    const mergedAt = item.mergedAt ?? item.merged_at;
    const headRef = item.headRef ?? item.head_ref ?? item.head?.ref ?? '';
    if (!Number.isInteger(number) || number < 1 || typeof mergedAt !== 'string') continue;
    if (!Number.isFinite(Date.parse(mergedAt))) throw new Error(`invalid merged timestamp for PR-${number}`);
    const metadata = metadataPlanCard(item);
    records.set(number, {
      number,
      mergedAt,
      headRef,
      planCard: metadata.value,
      planCardDeclared: metadata.declared,
    });
  }
  return [...records.values()].sort((left, right) => {
    const dateOrder = Date.parse(left.mergedAt) - Date.parse(right.mergedAt);
    return dateOrder || left.number - right.number;
  });
}

function isProgressAutomation(record) {
  return record.headRef === PROGRESS_BRANCH;
}

function resolvedPlanCard(record, plannedSet) {
  if (Number.isInteger(record.planCard) && plannedSet.has(record.planCard)) return record.planCard;
  if (record.planCardDeclared === true) return null;
  return plannedSet.has(record.number) ? record.number : null;
}

export function calculateProgress(plannedCards, mergedPullRequests) {
  const planned = sortedUniqueCards(plannedCards);
  if (!Array.isArray(mergedPullRequests)) throw new TypeError('mergedPullRequests must be an array');

  const plannedSet = new Set(planned);
  const merged = parseMergedPullRequests(mergedPullRequests).filter((record) => !isProgressAutomation(record));
  const matchedCards = new Set();
  let unmappedWork = 0;
  for (const record of merged) {
    const card = resolvedPlanCard(record, plannedSet);
    if (card === null) unmappedWork += 1;
    else matchedCards.add(card);
  }
  const gaps = planned.filter((card) => !matchedCards.has(card));
  const completed = matchedCards.size;
  const total = planned.length;

  return {
    completed,
    total,
    percentage: total === 0 ? 0 : Math.round((completed / total) * 100),
    latestMerged: merged.at(-1)
      ? { number: merged.at(-1).number, mergedAt: merged.at(-1).mergedAt }
      : null,
    nextCard: gaps[0] ?? null,
    gaps,
    unmappedWork,
  };
}

export function renderProgressSection(progress) {
  const { completed, total, percentage, latestMerged, nextCard, gaps = [], unmappedWork = 0 } = progress;
  if (![completed, total, percentage].every(Number.isInteger) || completed < 0 || total < 1 || completed > total) {
    throw new Error('invalid progress values');
  }

  const width = 20;
  const filled = Math.round((completed / total) * width);
  const bar = `${'█'.repeat(filled)}${'░'.repeat(width - filled)}`;
  const latest = latestMerged
    ? `#${latestMerged.number} · ${latestMerged.mergedAt}`
    : 'nenhuma';
  const next = nextCard ? cardLabel(nextCard) : 'fila sem lacunas';
  const gapSummary = gaps.length ? `${gaps.length} · primeira ${cardLabel(gaps[0])}` : 'nenhuma';

  return [
    PROGRESS_START,
    '## Progresso do plano',
    '',
    `**Cobertura observada:** ${completed}/${total} IDs do plano têm PR mergeada · ${percentage}%`,
    '',
    `\`${bar}\``,
    '',
    `- Última PR de trabalho mergeada: \`${latest}\``,
    `- IDs do plano sem PR correspondente: \`${gapSummary}\``,
    `- Próximo card sem correspondência: \`${next}\``,
    `- PRs mergeadas sem card correspondente: \`${unmappedWork}\``,
    '- Fonte: `.planning/queue/queue-*.md` e PRs mergeadas no GitHub.',
    '- Nota: a barra mede correspondência de integração por ID; PRs fora da fila devem declarar `Plan card: none` e a conclusão continua dependente da prova/ledger do plano.',
    PROGRESS_END,
  ].join('\n');
}

function markerPattern() {
  const start = PROGRESS_START.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const end = PROGRESS_END.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  return new RegExp(`${start}[\\s\\S]*?${end}`, 'g');
}

export function replaceProgressSection(readme, section) {
  if (typeof readme !== 'string' || typeof section !== 'string') throw new TypeError('README and section must be strings');
  const matches = readme.match(markerPattern()) ?? [];
  if (matches.length !== 1) throw new Error('README must contain exactly one plan progress section');
  return readme.replace(markerPattern(), section);
}

function parseArgs(argv) {
  const args = new Map();
  for (let index = 0; index < argv.length; index += 1) {
    const key = argv[index];
    if (!key.startsWith('--')) throw new Error(`unexpected argument: ${key}`);
    const value = argv[index + 1];
    if (!value || value.startsWith('--')) throw new Error(`missing value for ${key}`);
    args.set(key, value);
    index += 1;
  }
  return args;
}

function loadQueueTexts(queueDir) {
  const files = readdirSync(queueDir)
    .filter((file) => /^queue-.*\.md$/u.test(file))
    .sort();
  if (files.length === 0) throw new Error(`no queue files found in ${queueDir}`);
  return files.map((file) => readFileSync(resolve(queueDir, file), 'utf8'));
}

function run(argv) {
  const args = parseArgs(argv);
  const mergedFile = args.get('--merged-prs');
  if (!mergedFile) throw new Error('usage: node tools/plan-progress.mjs --merged-prs <json> [--queue-dir <dir>] [--write <readme>]');

  const queueDir = args.get('--queue-dir') ?? '.planning/queue';
  const readmePath = args.get('--write') ?? 'README.md';
  const planned = extractPlannedCards(loadQueueTexts(queueDir));
  const merged = parseMergedPullRequests(JSON.parse(readFileSync(mergedFile, 'utf8').replace(/^\uFEFF/u, '')));
  const progress = calculateProgress(planned, merged);
  const readme = readFileSync(readmePath, 'utf8');
  const updated = replaceProgressSection(readme, renderProgressSection(progress));

  if (args.has('--write') && updated !== readme) writeFileSync(readmePath, updated);
  process.stdout.write(`${JSON.stringify(progress)}\n`);
}

if (process.argv[1] && basename(process.argv[1]) === 'plan-progress.mjs') {
  try {
    run(process.argv.slice(2));
  } catch (error) {
    console.error(`plan progress error: ${error.message}`);
    process.exitCode = 1;
  }
}

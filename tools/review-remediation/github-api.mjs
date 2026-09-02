export const GITHUB_API_ENDPOINT = 'https://api.github.com';
export const MAX_GITHUB_RESPONSE_BYTES = 256 * 1024;

export class GithubApiError extends Error {
  constructor(code) {
    super(code);
    this.name = 'GithubApiError';
    this.code = code;
  }
}

function error(code) {
  return new GithubApiError(code);
}

function assertRepository(repository) {
  if (typeof repository !== 'string' || !/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repository)) throw error('GITHUB_REPOSITORY_INVALID');
}

function assertNumber(value, code) {
  if (!Number.isSafeInteger(value) || value <= 0) throw error(code);
}

function assertArray(value, code) {
  if (!Array.isArray(value)) throw error(code);
  return value;
}

export function createGithubApi({ token, repository, fetchImpl = globalThis.fetch }) {
  assertRepository(repository);
  if (typeof token !== 'string' || token.trim().length === 0) throw error('GITHUB_TOKEN_MISSING');
  if (typeof fetchImpl !== 'function') throw error('GITHUB_TRANSPORT_UNAVAILABLE');
  const repoPath = repository.split('/').map(encodeURIComponent).join('/');

  async function request(path, { allowNotFound = false } = {}) {
    let response;
    try {
      response = await fetchImpl(`${GITHUB_API_ENDPOINT}${path}`, {
        method: 'GET',
        headers: {
          Accept: 'application/vnd.github+json',
          Authorization: `Bearer ${token}`,
          'User-Agent': 'hank-review-remediation/1',
          'X-GitHub-Api-Version': '2022-11-28',
        },
      });
    } catch {
      throw error('GITHUB_NETWORK_ERROR');
    }
    if (response.status === 404 && allowNotFound) return null;
    if (!response || response.ok !== true || typeof response.text !== 'function') throw error(`GITHUB_HTTP_${response?.status ?? 'INVALID'}`);
    let text;
    try {
      text = await response.text();
    } catch {
      throw error('GITHUB_MALFORMED_RESPONSE');
    }
    if (typeof text !== 'string' || Buffer.byteLength(text, 'utf8') > MAX_GITHUB_RESPONSE_BYTES) throw error('GITHUB_RESPONSE_TOO_LARGE');
    try {
      return JSON.parse(text);
    } catch {
      throw error('GITHUB_MALFORMED_RESPONSE');
    }
  }

  async function getPullRequest(number) {
    assertNumber(number, 'GITHUB_PULL_REQUEST_INVALID');
    const result = await request(`/repos/${repoPath}/pulls/${number}`);
    if (!result || typeof result !== 'object') throw error('GITHUB_PULL_REQUEST_INVALID');
    return result;
  }

  async function getReviewComments(number) {
    assertNumber(number, 'GITHUB_PULL_REQUEST_INVALID');
    return assertArray(await request(`/repos/${repoPath}/pulls/${number}/comments?per_page=100&page=1`), 'GITHUB_REVIEW_COMMENTS_INVALID').slice(0, 100);
  }

  async function getIssueComments(number) {
    assertNumber(number, 'GITHUB_PULL_REQUEST_INVALID');
    return assertArray(await request(`/repos/${repoPath}/issues/${number}/comments?per_page=100&page=1`), 'GITHUB_ISSUE_COMMENTS_INVALID').slice(0, 100);
  }

  async function getPullRequestFiles(number) {
    assertNumber(number, 'GITHUB_PULL_REQUEST_INVALID');
    return assertArray(await request(`/repos/${repoPath}/pulls/${number}/files?per_page=100&page=1`), 'GITHUB_PULL_REQUEST_FILES_INVALID').slice(0, 100);
  }

  async function getCheckAnnotations(checkRunId) {
    assertNumber(checkRunId, 'GITHUB_CHECK_RUN_INVALID');
    return assertArray(await request(`/repos/${repoPath}/check-runs/${checkRunId}/annotations?per_page=100&page=1`), 'GITHUB_CHECK_ANNOTATIONS_INVALID').slice(0, 100);
  }

  async function getCheckRun(checkRunId) {
    assertNumber(checkRunId, 'GITHUB_CHECK_RUN_INVALID');
    const result = await request(`/repos/${repoPath}/check-runs/${checkRunId}`);
    if (!result || typeof result !== 'object') throw error('GITHUB_CHECK_RUN_INVALID');
    return result;
  }

  async function getBranch(branch) {
    if (typeof branch !== 'string' || branch.length === 0 || branch.length > 256) throw error('GITHUB_BRANCH_INVALID');
    return request(`/repos/${repoPath}/branches/${encodeURIComponent(branch)}`, { allowNotFound: true });
  }

  return {
    getPullRequest,
    getReviewComments,
    getCheckAnnotations,
    getCheckRun,
    getIssueComments,
    getPullRequestFiles,
    getBranch,
  };
}

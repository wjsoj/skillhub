// Single typed HTTP client for the SkillHub backend.
//
// Auth: a `X-Mock-User-Id` header simulates a session. In real
// deployment the JWT/API-token middleware kicks in, but the dev
// header path lets us drive E2E flows without a full login UX.

const STORAGE_USER_KEY = "skillhub.userId";
const STORAGE_USERNAME_KEY = "skillhub.username";

export function getMockUser(): { id: string; name: string } | null {
  const id = localStorage.getItem(STORAGE_USER_KEY);
  const name = localStorage.getItem(STORAGE_USERNAME_KEY) ?? "";
  return id ? { id, name } : null;
}

export function setMockUser(id: string, name: string) {
  localStorage.setItem(STORAGE_USER_KEY, id);
  localStorage.setItem(STORAGE_USERNAME_KEY, name);
}

export function clearMockUser() {
  localStorage.removeItem(STORAGE_USER_KEY);
  localStorage.removeItem(STORAGE_USERNAME_KEY);
}

export class ApiError extends Error {
  status: number;
  body: unknown;
  constructor(status: number, body: unknown, msg: string) {
    super(msg);
    this.status = status;
    this.body = body;
  }
}

export async function api<T>(
  path: string,
  init: RequestInit = {}
): Promise<T> {
  const user = getMockUser();
  const headers = new Headers(init.headers);
  headers.set("Accept", "application/json");
  if (init.body && !headers.has("Content-Type")) {
    headers.set("Content-Type", "application/json");
  }
  if (user) {
    headers.set("X-Mock-User-Id", user.id);
    if (user.name) headers.set("X-Mock-Username", user.name);
  }

  const res = await fetch(`/api/v1${path}`, { ...init, headers });
  const text = await res.text();
  const body: unknown = text ? safeJson(text) : null;
  if (!res.ok) {
    let msg = `HTTP ${res.status}`;
    if (body && typeof body === "object" && "error" in (body as object)) {
      msg = String((body as { error: unknown }).error);
    }
    throw new ApiError(res.status, body, msg);
  }
  return body as T;
}

function safeJson(s: string): unknown {
  try { return JSON.parse(s); } catch { return s; }
}

/* ────────────── typed endpoints ────────────── */

export interface Skill {
  id: string;
  namespace_id: string;
  namespace_slug: string;
  department_id: string | null;
  slug: string;
  display_name: string;
  description: string | null;
  visibility: "private" | "team" | "global";
  manifest: Record<string, unknown> & {
    version?: string;
    license?: string;
    author?: string;
    category?: string;
    entrypoint?: string;
    runtime?: Record<string, string>;
    inputs?: Array<{ name: string; type: string; required?: boolean; default?: unknown; description?: string }>;
    outputs?: Array<{ name: string; type: string; description?: string }>;
    dependencies?: string[];
    compliance?: Record<string, unknown>;
    files?: Array<{ path: string; size: number | null; kind: string }>;
    deprecated?: boolean;
    deprecation_note?: string;
  };
  readme: string | null;
  install_command: string | null;
  repository_url: string | null;
  tags: string[];
  downloads: number;
  install_count: number;
  stars: number;
  created_at: string;
  updated_at: string;
}

export const listSkills = () => api<Skill[]>(`/skills`);
export const getSkill = (id: string) => api<Skill>(`/skills/${id}`);

export interface DuplicateCheckBody {
  display_name: string;
  slug: string;
  description?: string;
  readme?: string;
  manifest?: Record<string, unknown>;
  tags?: string[];
  exclude_skill_id?: string;
}

export interface DuplicateCandidate {
  hit: {
    skill_id: string;
    namespace_slug: string;
    slug: string;
    display_name: string;
    description: string | null;
    score: number;
    matched_on: string[];
  };
  confidence: "high" | "medium" | "low";
  suggested_action: "use_existing" | "review" | "inform";
}

export interface DuplicateReport {
  query_hash: string;
  model: string;
  candidates: DuplicateCandidate[];
}

export const checkDuplicate = (body: DuplicateCheckBody) =>
  api<DuplicateReport>(`/skills/check-duplicate`, {
    method: "POST",
    body: JSON.stringify(body),
  });

export interface Collaborator {
  user_id: string;
  role: "maintainer" | "writer" | "reader";
  added_by: string;
  added_at: string;
}

export const listCollaborators = (skillId: string) =>
  api<Collaborator[]>(`/skills/${skillId}/collaborators`);

export const addCollaborator = (
  skillId: string,
  user_id: string,
  role: "maintainer" | "writer" | "reader"
) =>
  api<Collaborator>(`/skills/${skillId}/collaborators`, {
    method: "POST",
    body: JSON.stringify({ user_id, role }),
  });

export interface IterationJob {
  id: string;
  state:
    | "queued"
    | "running"
    | "succeeded"
    | "failed"
    | "cancelled"
    | "submitted";
  agent: string;
  intent: string;
  started_at: string | null;
  finished_at: string | null;
  submitted_proposal: string | null;
  error: string | null;
}

export const listIterations = (skillId: string) =>
  api<IterationJob[]>(`/skills/${skillId}/iterations`);

export const openIteration = (
  skillId: string,
  body: { agent: string; intent: string; base_version_id?: string | null }
) =>
  api<IterationJob>(`/skills/${skillId}/iterations`, {
    method: "POST",
    body: JSON.stringify(body),
  });

export const pushPatch = (
  skillId: string,
  jobId: string,
  body: {
    seq: number;
    path: string;
    op: "write" | "delete" | "rename";
    data_b64?: string;
    new_path?: string;
  }
) =>
  api<{ patch_id: string }>(`/skills/${skillId}/iterations/${jobId}/patches`, {
    method: "POST",
    body: JSON.stringify(body),
  });

export const runIterationTests = (
  skillId: string,
  jobId: string,
  command: string
) =>
  api<{
    command: string;
    exit_code: number;
    duration_ms: number;
    timed_out: boolean;
    stdout: string;
    stderr: string;
  }>(`/skills/${skillId}/iterations/${jobId}/run-tests`, {
    method: "POST",
    body: JSON.stringify({ command }),
  });

export const submitIteration = (
  skillId: string,
  jobId: string,
  body: { target_version: string; summary?: string; title: string; body?: string }
) =>
  api<{
    draft_id: string;
    proposal_id: string;
    job_state: string;
  }>(`/skills/${skillId}/iterations/${jobId}/submit`, {
    method: "POST",
    body: JSON.stringify(body),
  });

export interface Department {
  id: string;
  org_id: string;
  parent_id: string | null;
  slug: string;
  name: string;
}

export const listDepartments = (orgId: string) =>
  api<Department[]>(`/orgs/${orgId}/departments`);

export const createDepartment = (
  orgId: string,
  body: { slug: string; name: string; parent_id?: string | null }
) =>
  api<Department>(`/orgs/${orgId}/departments`, {
    method: "POST",
    body: JSON.stringify(body),
  });

export const addDepartmentMember = (
  deptId: string,
  body: { user_id: string; role: "director" | "manager" | "member" }
) =>
  api<unknown>(`/departments/${deptId}/members`, {
    method: "POST",
    body: JSON.stringify(body),
  });

export const createGrant = (body: {
  grantee_department_id?: string | null;
  grantee_user_id?: string | null;
  target_department_id?: string | null;
  target_namespace_id?: string | null;
  target_skill_id?: string | null;
  scope: "read" | "write" | "admin";
  reason: string;
  expires_at?: string | null;
}) =>
  api<{ id: string }>(`/grants`, {
    method: "POST",
    body: JSON.stringify(body),
  });

export interface Proposal {
  id: string;
  skill_id: string;
  draft_id: string;
  state:
    | "open"
    | "changes_requested"
    | "approved"
    | "rejected"
    | "merged"
    | "withdrawn";
  title: string;
  body: string | null;
  opened_by: string;
  merged_version_id: string | null;
}

export const listProposals = (skillId: string) =>
  api<Proposal[]>(`/skills/${skillId}/proposals`);

export const createDraft = (
  skillId: string,
  body: {
    base_version_id?: string | null;
    target_version: string;
    manifest: Record<string, unknown>;
    summary?: string;
  }
) =>
  api<{ draft_id: string }>(`/skills/${skillId}/drafts`, {
    method: "POST",
    body: JSON.stringify(body),
  });

export const openProposal = (
  skillId: string,
  body: { draft_id: string; title: string; body?: string }
) =>
  api<Proposal>(`/skills/${skillId}/proposals`, {
    method: "POST",
    body: JSON.stringify(body),
  });

export const decideProposal = (
  skillId: string,
  pid: string,
  state: Proposal["state"]
) =>
  api<Proposal>(`/skills/${skillId}/proposals/${pid}/decide`, {
    method: "POST",
    body: JSON.stringify({ state }),
  });

export const reviewProposal = (
  skillId: string,
  pid: string,
  body: { verdict: "comment" | "approve" | "request_changes" | "reject"; body?: string }
) =>
  api<{ review_id: string }>(`/skills/${skillId}/proposals/${pid}/reviews`, {
    method: "POST",
    body: JSON.stringify(body),
  });

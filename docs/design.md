# SkillHub — 四大特性设计

本文覆盖：(A) 语义查重 · (B) 协作与版本管理 · (C) AI 自迭代 harness · (D) 部门级精确权限。
四个能力相互交织——查重为发布把关，版本/协作承载提案流，harness 把 AI 产物落入提案流，
权限贯穿所有读写动作。

---

## A. 语义查重

### 目标
用户上传/创建 skill 时，**不靠名字相似**，而是用语义嵌入向量在已有 skill 中召回 top-K
相似项，提示是否已存在等价能力。

### 数据流
```
publish/draft create
       │
       ▼
content normalizer ──► Embedder ──► embedding (f32[dim])
       │                                    │
       │                                    ▼
       │                            skill_embeddings (pgvector)
       │                                    │
       ▼                                    ▼
DuplicateDetector ────── trigram 召回 + cosine top-K ────► 候选列表
       │
       ▼
   返回给前端 / CLI / AI agent
```

### 关键决策
- **Embedder 接口可插拔**：`HttpEmbedder`（OpenAI 兼容端点，支持 Ollama / vLLM / 自建）
  与 `StubEmbedder`（确定性哈希降维，纯本地，CI/离线开发用）。
- **存储**：`pgvector` 扩展；`skill_embeddings.embedding vector(N)`；`ivfflat` 索引（cosine）。
- **混合召回**：先 `pg_trgm` 词面相似过滤大盘，再向量精排，避免向量索引在小规模下漂移。
- **可见性 + 部门权限过滤**：召回结果必须经过 `PolicyEvaluator`，跨部门候选默认不出现，
  除非显式 grant。
- **触发点**：
  - 显式 `POST /skills/check-duplicate`（创建前手动调用）。
  - `POST /skills`（隐式预检，命中高分阈值时返回 409 + 候选列表，可强制覆盖）。
  - 版本 publish / proposal 合并时再次校验。

---

## B. 协作与版本管理

### 模型
- **Skill** 是聚合根；下挂 `SkillVersion`（已发布）、`VersionDraft`（开发中）、
  `VersionProposal`（待评审）。
- **Collaborator**：与 namespace 成员**独立**——namespace 决定"谁能创建 skill"，
  collaborator 决定"谁能动这个 skill"。角色：`maintainer / writer / reader`。
- **Lineage**：`skill_versions.parent_version_id` 形成 DAG；`lineage_id` 标识
  同一逻辑演化分支（fork 时换新 lineage）。
- **乐观锁**：`skills.revision` 与 `skill_versions.etag` 用于条件写。

### 状态机
```
Draft ──open proposal──► Proposal(pending)
                              │
                ┌─────────────┼─────────────┐
                ▼             ▼             ▼
           approved       changes_req     rejected
                │
              merge ──► SkillVersion (status=approved)
                              │
                          (yank 可选)
```

### 权限
- `maintainer` 可合并 proposal、yank 版本、改协作者
- `writer` 可开 draft / proposal、push patch
- `reader` 只读
- namespace admin 始终可托管；super_admin 全局

### 活动时间线
每个动作产生一条 `activity_events`（actor, verb, target, payload），驱动前端 timeline
与通知。

---

## C. AI 自迭代 Harness

### 目标
让 AI agent 像协作者一样，**安全、可观测**地迭代 skill：拉取基线 → 局部修改 → 跑测试
→ 提交 proposal。

### 架构
```
agent (持 iteration-scoped token)
      │
      ▼
POST /skills/:id/iterations { base_version, agent, intent }
      │
      ▼  create IterationJob (queued)
JobRunner ──► SkillWorkspace (临时目录 + 解压包)
      │
      ├─ PATCH /iterations/:jid/files  (写入修改)
      ├─ POST  /iterations/:jid/run-tests
      ├─ GET   /iterations/:jid/logs
      │
      ▼  POST /iterations/:jid/submit
   captures diff vs base → 创建 VersionDraft + VersionProposal
```

### 状态机
```
queued → running → (succeeded | failed | cancelled) → submitted
```

### 安全
- 沙盒：子进程 + 临时目录 + CPU/内存/壁钟限制；网络默认禁止（白名单可配）。
- Token scope：`iteration:write` 仅允许操作 job 自身的 workspace 与提交。
- 部门策略：iteration 必须在 base skill 的部门 scope 内，跨部门同样需要 grant。
- 全部操作入 `activity_events` 与 `audit_entries`。

### AI 工效
- 状态/日志/diff 全部用稳定的 JSON 暴露，方便 agent 解析。
- `intent` 字段强制 agent 声明"为什么改"，进 proposal 描述与 audit。

---

## D. 部门级精确权限

### 模型
- **Organization** → **Department**（树状，closure table）→ **Namespace**。
- `user_department_memberships(user_id, department_id, role)` 决定用户在树上某节点的角色；
  role：`director / manager / member`。
- **继承**：用户在节点 N 的角色对 N 的所有子孙生效。
- **跨部门访问**：必须通过 `cross_scope_grants(grantee_dept_id, target_dept_id|namespace_id|skill_id, scope, reason, expires_at)` 显式授权。
- **可见性 + 权限分离**：`Visibility::Global` 让 skill 在所有部门可见，但写操作仍受
  collaborator + 部门 grant 约束。

### 策略评估
集中在 `skillhub-auth::policy`：
```rust
pub trait PolicyEvaluator {
    fn evaluate(&self, ctx: &PermissionCtx, action: Action, target: &Target) -> Decision;
}
```
- 默认拒绝。
- 评估顺序：super_admin → skill collaborator → namespace member → 部门继承角色 → cross-scope grant → visibility 兜底。
- 每次 `Decision::Deny` 写 `audit_entries.action='access.denied'`，便于审计与排查"乱窜"事故。

### 中间件
- `RequirePrincipal`：解析 JWT / API token → `Principal`。
- `WithDepartmentScope`：一次性把用户的"可见部门集合 + grant"塞进 request 扩展。
- 每个写路由调用 `policy.require(action, target)`，违反即 403。

---

## 横向：审计与可观测
- 所有授权决定、版本状态变更、iteration 关键节点全部写 `audit_entries` 与 `activity_events`。
- Prometheus 计数：`skillhub_duplicate_check_total`, `skillhub_iteration_jobs{state}`,
  `skillhub_policy_denies_total{action}`。
- `tracing` span 携带 `principal.user_id`, `department_scope`, `skill_id`，方便排查跨部门事故。

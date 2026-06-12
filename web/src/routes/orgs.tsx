import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ChevronRight, ChevronDown, Plus, Loader2 } from "lucide-react";
import { PageHeader } from "@/components/ui/PageHeader";
import { Badge } from "@/components/ui/Badge";
import { addDepartmentMember, createDepartment, listDepartments, type Department } from "@/lib/api";
import { cn } from "@/lib/cn";

const DEFAULT_ORG_ID = "10000000-0000-0000-0000-0000000000aa";

export function OrgsPage() {
  const qc = useQueryClient();
  const q = useQuery({ queryKey: ["departments", DEFAULT_ORG_ID], queryFn: () => listDepartments(DEFAULT_ORG_ID) });

  const tree = q.data ? buildTree(q.data) : [];
  const [selected, setSelected] = useState<Department | null>(null);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [adding, setAdding] = useState(false);

  return (
    <>
      <PageHeader
        eyebrow="Org"
        title={
          <>
            Who can <span className="serif-em">see what.</span>
          </>
        }
        description="Departments are the basic unit of access. People belong to a department, skills belong to a department through their namespace, and crossing the line takes a written grant."
        actions={
          <button
            onClick={() => setAdding((v) => !v)}
            className="btn btn-secondary"
          >
            <Plus size={14} /> {adding ? "Cancel" : "Add a department"}
          </button>
        }
      />

      {adding && (
        <div className="mb-10 p-5 rounded-2xl" style={{ background: "var(--surface-2)" }}>
          <CreateDept
            orgId={DEFAULT_ORG_ID}
            parents={q.data ?? []}
            onCreated={() => {
              setAdding(false);
              qc.invalidateQueries({ queryKey: ["departments", DEFAULT_ORG_ID] });
            }}
          />
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-[minmax(0,1fr)_minmax(0,360px)] gap-12">
        {/* Tree */}
        <div>
          <h3 className="text-[15px] font-semibold mb-4">Departments</h3>
          {q.isLoading && <Loader2 size={18} className="animate-spin" style={{ color: "var(--fg-muted)" }} />}
          {tree.length > 0 && (
            <Tree
              nodes={tree}
              selectedId={selected?.id ?? null}
              expanded={expanded}
              onToggle={(id) => {
                setExpanded((prev) => {
                  const n = new Set(prev);
                  if (n.has(id)) n.delete(id);
                  else n.add(id);
                  return n;
                });
              }}
              onSelect={setSelected}
            />
          )}
        </div>

        {/* Members */}
        <aside className="lg:sticky lg:top-24 self-start">
          {!selected ? (
            <div className="py-12 text-center">
              <p className="text-[14px]" style={{ color: "var(--fg-muted)" }}>
                Pick a department to manage members.
              </p>
            </div>
          ) : (
            <MembersPanel dept={selected} />
          )}
        </aside>
      </div>
    </>
  );
}

interface TreeNode { dept: Department; children: TreeNode[]; }

function buildTree(depts: Department[]): TreeNode[] {
  const map = new Map<string, TreeNode>(depts.map((d) => [d.id, { dept: d, children: [] }]));
  const roots: TreeNode[] = [];
  for (const d of depts) {
    const node = map.get(d.id)!;
    if (d.parent_id && map.has(d.parent_id)) map.get(d.parent_id)!.children.push(node);
    else roots.push(node);
  }
  return roots;
}

function Tree({
  nodes, selectedId, expanded, onToggle, onSelect, depth = 0,
}: {
  nodes: TreeNode[];
  selectedId: string | null;
  expanded: Set<string>;
  onToggle: (id: string) => void;
  onSelect: (d: Department) => void;
  depth?: number;
}) {
  return (
    <ul className="space-y-1">
      {nodes.map((n) => {
        const isOpen = expanded.has(n.dept.id) || depth === 0;
        const isSelected = selectedId === n.dept.id;
        const hasKids = n.children.length > 0;
        return (
          <li key={n.dept.id}>
            <div
              className={cn("flex items-center gap-2 rounded-lg")}
              style={{ paddingLeft: `${depth * 22}px` }}
              data-testid="dept-node"
            >
              <button
                aria-label={isOpen ? "Collapse" : "Expand"}
                onClick={() => hasKids && onToggle(n.dept.id)}
                className="flex items-center justify-center w-5 h-5 rounded-full"
                style={{ color: hasKids ? "var(--fg-muted)" : "transparent", cursor: hasKids ? "pointer" : "default" }}
              >
                {hasKids && (isOpen ? <ChevronDown size={13} /> : <ChevronRight size={13} />)}
              </button>
              <button
                onClick={() => onSelect(n.dept)}
                className="flex-1 flex items-center gap-2 py-2 px-2 rounded-lg text-left transition-colors"
                style={{
                  background: isSelected ? "var(--accent-soft)" : "transparent",
                  color: isSelected ? "var(--accent-soft-fg)" : "var(--fg)",
                  cursor: "pointer",
                }}
              >
                <span className="text-[15px] font-medium">{n.dept.name}</span>
                <span className="text-[12px] font-mono" style={{ color: "var(--fg-faint)" }}>
                  {n.dept.slug}
                </span>
              </button>
            </div>
            {isOpen && n.children.length > 0 && (
              <Tree
                nodes={n.children}
                selectedId={selectedId}
                expanded={expanded}
                onToggle={onToggle}
                onSelect={onSelect}
                depth={depth + 1}
              />
            )}
          </li>
        );
      })}
    </ul>
  );
}

function CreateDept({
  orgId, parents, onCreated,
}: {
  orgId: string;
  parents: Department[];
  onCreated: () => void;
}) {
  const [slug, setSlug] = useState("");
  const [name, setName] = useState("");
  const [parent, setParent] = useState("");

  const create = useMutation({
    mutationFn: () => createDepartment(orgId, { slug, name, parent_id: parent || null }),
    onSuccess: () => { setSlug(""); setName(""); setParent(""); onCreated(); },
  });

  return (
    <form
      className="flex flex-col sm:flex-row gap-2"
      onSubmit={(e) => { e.preventDefault(); if (slug && name) create.mutate(); }}
    >
      <select className="select w-full sm:w-[160px]" value={parent} onChange={(e) => setParent(e.target.value)} data-testid="dept-parent">
        <option value="">(top level)</option>
        {parents.map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}
      </select>
      <input className="input input-mono w-full sm:w-[160px]" value={slug} onChange={(e) => setSlug(e.target.value)} placeholder="slug" data-testid="dept-slug" />
      <input className="input w-full sm:flex-1" value={name} onChange={(e) => setName(e.target.value)} placeholder="Department name" data-testid="dept-name" />
      <button className="btn btn-primary" disabled={!slug || !name || create.isPending} data-testid="dept-create">
        {create.isPending ? <Loader2 size={14} className="animate-spin" /> : "Add"}
      </button>
    </form>
  );
}

function MembersPanel({ dept }: { dept: Department }) {
  const [userId, setUserId] = useState("");
  const [role, setRole] = useState<"director" | "manager" | "member">("member");
  const add = useMutation({
    mutationFn: () => addDepartmentMember(dept.id, { user_id: userId, role }),
    onSuccess: () => setUserId(""),
  });

  return (
    <div>
      <div className="text-[13px] mb-1" style={{ color: "var(--fg-subtle)" }}>Selected</div>
      <h3 className="display-2 mb-1">{dept.name}</h3>
      <div className="font-mono text-[11.5px] mb-7" style={{ color: "var(--fg-faint)" }}>{dept.id}</div>

      <h4 className="text-[14px] font-semibold mb-3">Add a member</h4>
      <form
        className="space-y-3"
        onSubmit={(e) => { e.preventDefault(); if (userId) add.mutate(); }}
      >
        <input
          className="input input-mono"
          value={userId}
          onChange={(e) => setUserId(e.target.value)}
          placeholder="user uuid"
          data-testid="member-user"
        />
        <select className="select" value={role} onChange={(e) => setRole(e.target.value as typeof role)} data-testid="member-role">
          <option value="member">Member</option>
          <option value="manager">Manager</option>
          <option value="director">Director</option>
        </select>
        <div className="flex items-center gap-3">
          <button className="btn btn-primary" disabled={!userId || add.isPending} data-testid="member-add">
            {add.isPending ? <><Loader2 size={14} className="animate-spin" /> Adding…</> : "Add"}
          </button>
          {add.isSuccess && <Badge tone="ok">added</Badge>}
          {add.error && <Badge tone="bad">{(add.error as Error).message}</Badge>}
        </div>
      </form>
    </div>
  );
}

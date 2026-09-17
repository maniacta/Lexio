import { useCallback, useEffect, useMemo, useState } from "react";
import type { AuditFacets, AuditLogEntry, AuditLogPage } from "../../types";
import { api } from "../../api/client";
import "./AuditLogTab.css";

/** Matches the backend's default; the backend clamps anything larger. */
const PAGE_SIZE = 50;

const LEVEL_LABELS: Record<string, string> = {
  info: "信息",
  warn: "警告",
  error: "错误",
};

interface Filters {
  level: string;
  source: string;
  category: string;
  search: string;
  since: string;
  until: string;
}

const EMPTY_FILTERS: Filters = {
  level: "",
  source: "",
  category: "",
  search: "",
  since: "",
  until: "",
};

/** Format an RFC3339 UTC stamp in the viewer's local time. */
function formatTimestamp(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString();
}

/** `datetime-local` gives a local wall-clock string; the API wants UTC. */
function toRfc3339Utc(local: string): string | undefined {
  if (!local) return undefined;
  const date = new Date(local);
  if (Number.isNaN(date.getTime())) return undefined;
  return date.toISOString();
}

/** Collapse a JSON summary to one readable line, or keep raw text as-is. */
function formatSummary(raw: string | null): string {
  if (!raw) return "";
  try {
    const parsed: unknown = JSON.parse(raw);
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
      return Object.entries(parsed as Record<string, unknown>)
        .map(([k, v]) => `${k}=${typeof v === "string" ? v : JSON.stringify(v)}`)
        .join("  ");
    }
    return String(parsed);
  } catch {
    return raw;
  }
}

export default function AuditLogTab() {
  const [filters, setFilters] = useState<Filters>(EMPTY_FILTERS);
  // `applied` lags `filters` so typing does not fire a request per keystroke.
  const [applied, setApplied] = useState<Filters>(EMPTY_FILTERS);
  const [page, setPage] = useState(0);
  const [data, setData] = useState<AuditLogPage | null>(null);
  const [facets, setFacets] = useState<AuditFacets | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<string | null>(null);

  const query = useMemo(
    () => ({
      level: applied.level || undefined,
      source: applied.source || undefined,
      category: applied.category || undefined,
      search: applied.search.trim() || undefined,
      since: toRfc3339Utc(applied.since),
      until: toRfc3339Utc(applied.until),
      limit: PAGE_SIZE,
      offset: page * PAGE_SIZE,
    }),
    [applied, page],
  );

  const load = useCallback(
    async (signal?: AbortSignal) => {
      setLoading(true);
      setError(null);
      try {
        const next = await api.audit.listLogs(query, signal);
        if (!signal?.aborted) setData(next);
      } catch (e: unknown) {
        if (signal?.aborted) return;
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        if (!signal?.aborted) setLoading(false);
      }
    },
    [query],
  );

  useEffect(() => {
    const controller = new AbortController();
    void load(controller.signal);
    return () => controller.abort();
  }, [load]);

  // Facet options come from the trail itself, so the dropdown can only offer a
  // value some event has actually used. Failure here is cosmetic: the free-text
  // search still works, so it must not surface as a page error.
  useEffect(() => {
    const controller = new AbortController();
    api.audit
      .facets(controller.signal)
      .then((f) => {
        if (!controller.signal.aborted) setFacets(f);
      })
      .catch(() => undefined);
    return () => controller.abort();
  }, []);

  const applyFilters = () => {
    setPage(0);
    setApplied(filters);
  };

  const resetFilters = () => {
    setFilters(EMPTY_FILTERS);
    setApplied(EMPTY_FILTERS);
    setPage(0);
  };

  const total = data?.total ?? 0;
  const pageCount = Math.max(1, Math.ceil(total / PAGE_SIZE));
  const hasFilters = Object.values(applied).some((v) => v !== "");

  return (
    <div className="settings-tab audit-tab">
      <p className="tab-desc">
        这里记录应用在本机做过的操作。时间戳由后端生成（服务端时间），前端上报的时间仅作参考、
        不参与排序与保留期计算，因此无法被客户端改写。
      </p>

      <div className="setting-group audit-filters">
        <div className="audit-filter-grid">
          <label className="audit-field">
            <span>级别</span>
            <select
              value={filters.level}
              onChange={(e) => setFilters({ ...filters, level: e.target.value })}
            >
              <option value="">全部</option>
              {(facets?.levels ?? ["info", "warn", "error"]).map((l) => (
                <option key={l} value={l}>
                  {LEVEL_LABELS[l] ?? l}
                </option>
              ))}
            </select>
          </label>

          <label className="audit-field">
            <span>来源</span>
            <select
              value={filters.source}
              onChange={(e) => setFilters({ ...filters, source: e.target.value })}
            >
              <option value="">全部</option>
              {(facets?.sources ?? []).map((s) => (
                <option key={s} value={s}>
                  {s}
                </option>
              ))}
            </select>
          </label>

          <label className="audit-field">
            <span>类别</span>
            <select
              value={filters.category}
              onChange={(e) => setFilters({ ...filters, category: e.target.value })}
            >
              <option value="">全部</option>
              {(facets?.categories ?? []).map((c) => (
                <option key={c} value={c}>
                  {c}
                </option>
              ))}
            </select>
          </label>

          <label className="audit-field">
            <span>起始时间</span>
            <input
              type="datetime-local"
              value={filters.since}
              onChange={(e) => setFilters({ ...filters, since: e.target.value })}
            />
          </label>

          <label className="audit-field">
            <span>结束时间</span>
            <input
              type="datetime-local"
              value={filters.until}
              onChange={(e) => setFilters({ ...filters, until: e.target.value })}
            />
          </label>

          <label className="audit-field audit-field-wide">
            <span>搜索</span>
            <input
              type="text"
              placeholder="动作 / 路径 / 错误信息"
              value={filters.search}
              onChange={(e) => setFilters({ ...filters, search: e.target.value })}
              onKeyDown={(e) => {
                if (e.key === "Enter") applyFilters();
              }}
            />
          </label>
        </div>

        <div className="setting-actions audit-filter-actions">
          <button className="btn-secondary" onClick={resetFilters} disabled={!hasFilters && !filters.search}>
            重置
          </button>
          <button className="btn-primary" onClick={applyFilters} disabled={loading}>
            查询
          </button>
        </div>
      </div>

      {error && (
        <div className="settings-error audit-error">
          {error}
          <button className="btn-secondary" onClick={() => void load()}>
            重试
          </button>
        </div>
      )}

      {data && !error && (
        <>
          <div className="audit-meta">
            <span>
              共 {total} 条{hasFilters ? "（已筛选）" : ""}
            </span>
            <span className="audit-page-controls">
              <button
                className="btn-secondary"
                onClick={() => setPage((p) => Math.max(0, p - 1))}
                disabled={page === 0 || loading}
              >
                上一页
              </button>
              <span className="audit-page-indicator">
                {page + 1} / {pageCount}
              </span>
              <button
                className="btn-secondary"
                onClick={() => setPage((p) => p + 1)}
                disabled={page + 1 >= pageCount || loading}
              >
                下一页
              </button>
            </span>
          </div>

          {data.logs.length === 0 ? (
            <p className="audit-empty">{hasFilters ? "没有匹配的记录。" : "暂无审计记录。"}</p>
          ) : (
            <ul className="audit-list">
              {data.logs.map((entry) => (
                <AuditRow
                  key={entry.id}
                  entry={entry}
                  expanded={expanded === entry.id}
                  onToggle={() => setExpanded(expanded === entry.id ? null : entry.id)}
                />
              ))}
            </ul>
          )}
        </>
      )}

      {loading && !data && <p className="audit-empty">加载中...</p>}
    </div>
  );
}

interface RowProps {
  entry: AuditLogEntry;
  expanded: boolean;
  onToggle: () => void;
}

function AuditRow({ entry, expanded, onToggle }: RowProps) {
  const title = entry.user_action || entry.action;
  // A recorded timestamp that is not the server's own is worth flagging, so a
  // reader never mistakes the client value for the authoritative one.
  const clientSkew =
    entry.client_timestamp && entry.client_timestamp !== entry.timestamp
      ? entry.client_timestamp
      : null;

  return (
    <li className={`audit-row ${expanded ? "expanded" : ""}`}>
      <button className="audit-row-summary" onClick={onToggle} aria-expanded={expanded}>
        <span className={`audit-level audit-level-${entry.level}`}>
          {LEVEL_LABELS[entry.level] ?? entry.level}
        </span>
        <span className="audit-row-main">
          <span className="audit-row-title">{title}</span>
          <span className="audit-row-sub">
            {entry.category} · {entry.action} · {entry.source}
            {entry.status_code !== null ? ` · ${entry.status_code}` : ""}
            {entry.duration_ms !== null ? ` · ${entry.duration_ms}ms` : ""}
          </span>
        </span>
        <span className="audit-row-time">{formatTimestamp(entry.timestamp)}</span>
      </button>

      {expanded && (
        <dl className="audit-detail">
          <dt>服务端时间</dt>
          <dd>{entry.timestamp}</dd>

          {clientSkew && (
            <>
              <dt>客户端上报时间（仅供参考）</dt>
              <dd>{clientSkew}</dd>
            </>
          )}

          {entry.method && (
            <>
              <dt>请求</dt>
              <dd>
                {entry.method} {entry.path}
              </dd>
            </>
          )}

          {entry.params_summary && (
            <>
              <dt>参数</dt>
              <dd className="audit-monospace">{formatSummary(entry.params_summary)}</dd>
            </>
          )}

          {entry.result_summary && (
            <>
              <dt>结果</dt>
              <dd className="audit-monospace">{formatSummary(entry.result_summary)}</dd>
            </>
          )}

          {entry.error_message && (
            <>
              <dt>错误</dt>
              <dd className="audit-error-text">{entry.error_message}</dd>
            </>
          )}
        </dl>
      )}
    </li>
  );
}
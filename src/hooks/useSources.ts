import { useState, useEffect, useCallback, useRef } from "react";
import type { Source } from "../types";
import { api } from "../api/client";
import { isAbortError } from "../utils/errors";

export function useSources() {
  const [sources, setSources] = useState<Source[]>([]);
  const [loading, setLoading] = useState(true);
  const abortRef = useRef<AbortController | null>(null);

  const refresh = useCallback(async () => {
    abortRef.current?.abort();
    const ac = new AbortController();
    abortRef.current = ac;
    setLoading(true);
    try {
      const data = await api.sources.list(undefined, undefined, ac.signal);
      if (!ac.signal.aborted) setSources(data);
    } catch (err) {
      if (!isAbortError(err)) {
        console.error("Failed to load sources:", err);
      }
    } finally {
      if (!ac.signal.aborted) setLoading(false);
    }
  }, []);

  const toggleHidden = useCallback(async (id: string, hidden: boolean) => {
    await api.sources.toggleHidden(id, hidden);
    setSources((prev) =>
      prev.map((s) => (s.id === id ? { ...s, hidden } : s))
    );
  }, []);

  useEffect(() => {
    refresh();
    return () => abortRef.current?.abort();
  }, [refresh]);

  return { sources, loading, refresh, toggleHidden };
}

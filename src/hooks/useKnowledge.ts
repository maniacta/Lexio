import { useState, useEffect, useCallback, useRef } from "react";
import type { KnowledgePoint } from "../types";
import { api } from "../api/client";
import { isAbortError } from "../utils/errors";

export function useKnowledge() {
  const [kps, setKps] = useState<KnowledgePoint[]>([]);
  const [loading, setLoading] = useState(true);
  const abortRef = useRef<AbortController | null>(null);

  const refresh = useCallback(async () => {
    abortRef.current?.abort();
    const ac = new AbortController();
    abortRef.current = ac;
    setLoading(true);
    try {
      const data = await api.knowledge.list(undefined, undefined, ac.signal);
      if (!ac.signal.aborted) setKps(data);
    } catch (err) {
      if (!isAbortError(err)) {
        console.error("Failed to load knowledge points:", err);
      }
    } finally {
      if (!ac.signal.aborted) setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
    return () => abortRef.current?.abort();
  }, [refresh]);

  return { kps, loading, refresh };
}

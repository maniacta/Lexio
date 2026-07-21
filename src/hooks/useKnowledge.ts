import { useState, useEffect, useCallback } from "react";
import type { KnowledgePoint } from "../types";
import { api } from "../api/client";

export function useKnowledge() {
  const [kps, setKps] = useState<KnowledgePoint[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const data = await api.knowledge.list();
      setKps(data);
    } catch (err) {
      console.error("Failed to load knowledge points:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  return { kps, loading, refresh };
}

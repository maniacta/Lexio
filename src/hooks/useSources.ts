import { useState, useEffect, useCallback } from "react";
import type { Source } from "../types";
import { api } from "../api/client";

export function useSources() {
  const [sources, setSources] = useState<Source[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const data = await api.sources.list();
      setSources(data);
    } catch (err) {
      console.error("Failed to load sources:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  const toggleHidden = useCallback(async (id: string, hidden: boolean) => {
    await api.sources.toggleHidden(id, hidden);
    setSources((prev) =>
      prev.map((s) => (s.id === id ? { ...s, hidden } : s))
    );
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  return { sources, loading, refresh, toggleHidden };
}

import { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";

import { listPageGuidance, updatePageGuidance } from "../api/commands";
import type { PageGuidanceRecord } from "../types/guidance";
import { pageDescription, type PageKey } from "./pageDescriptions";

interface GuidanceContextValue {
  guidance: Readonly<Record<string, string>>;
  loading: boolean;
  error: string | null;
  save: (pageKey: PageKey, guidance: string | null) => Promise<PageGuidanceRecord>;
  reload: () => Promise<void>;
}

const GuidanceContext = createContext<GuidanceContextValue>({
  guidance: {},
  loading: false,
  error: null,
  save: async () => { throw new Error("Guidance is unavailable outside the authenticated application."); },
  reload: async () => undefined,
});

export function PageGuidanceProvider({ children }: { children: React.ReactNode }) {
  const [guidance, setGuidance] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setGuidance(toGuidanceMap(await listPageGuidance()));
    } catch {
      setError("Guidance could not be loaded. Standard page descriptions remain available.");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { void reload(); }, [reload]);

  const save = useCallback(async (pageKey: PageKey, value: string | null) => {
    const record = await updatePageGuidance({ pageKey, guidance: value });
    setGuidance((current) => {
      const next = { ...current };
      if (record.guidance) next[record.pageKey] = record.guidance;
      else delete next[record.pageKey];
      return next;
    });
    return record;
  }, []);

  const value = useMemo(() => ({ guidance, loading, error, save, reload }), [error, guidance, loading, reload, save]);
  return <GuidanceContext.Provider value={value}>{children}</GuidanceContext.Provider>;
}

export function PageDescription({ pageKey }: { pageKey: PageKey }) {
  const localGuidance = useContext(GuidanceContext).guidance[pageKey];
  return <PageDescriptionContent pageKey={pageKey} guidance={localGuidance} />;
}

export function PageDescriptionContent({ pageKey, guidance }: { pageKey: PageKey; guidance?: string }) {
  const standard = pageDescription(pageKey);
  return <>
    <p className="page-summary" lang="th">{standard.description}</p>
    {guidance && <p className="page-guidance"><strong>Guidance</strong><span>{guidance}</span></p>}
  </>;
}

export function usePageGuidance() {
  return useContext(GuidanceContext);
}

export function toGuidanceMap(records: PageGuidanceRecord[]): Record<string, string> {
  return Object.fromEntries(records.flatMap((record) => record.guidance ? [[record.pageKey, record.guidance]] : []));
}

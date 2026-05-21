"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { useCurrentUser } from "@/hooks/use-current-user";
import {
  cancelBackfillJob,
  createBackfillJob,
  getBackfillJobs,
  getBackfillRepos,
  getBackfillPdsSummary,
  flushBackfillDetails,
  flushAllBackfillDetails,
  getLexicons,
} from "@/lib/api";
import type {
  BackfillJob,
  BackfillRepoEntry,
  PdsSummaryEntry,
  BackfillEvent,
  BlueskyProfile,
} from "@/types/backfill";
import { CheckCircle2, ChevronRight, Circle, Loader2 } from "lucide-react";
import { SiteHeader } from "@/components/site-header";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { Badge } from "@/components/ui/badge";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { Button } from "@/components/ui/button";
import {
  Combobox,
  ComboboxContent,
  ComboboxEmpty,
  ComboboxInput,
  ComboboxItem,
  ComboboxList,
} from "@/components/ui/combobox";
import {
  ResponsiveDialog,
  ResponsiveDialogClose,
  ResponsiveDialogContent,
  ResponsiveDialogDescription,
  ResponsiveDialogFooter,
  ResponsiveDialogHeader,
  ResponsiveDialogTitle,
  ResponsiveDialogTrigger,
} from "@/components/ui/responsive-dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Sheet,
  SheetContent,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

const PROGRESS_PHASES = [
  "discovering_repos",
  "resolving_pds",
  "fetching_records",
] as const;

function statusBadge(job: BackfillJob) {
  switch (job.status) {
    case "completed":
      return (
        <Badge className="bg-emerald-500/15 text-emerald-700 dark:text-emerald-400 hover:bg-emerald-500/25 border-emerald-500/20">
          completed
        </Badge>
      );
    case "failed":
      return <Badge variant="destructive">failed</Badge>;
    case "cancelled":
      return (
        <Badge className="bg-amber-500/15 text-amber-700 dark:text-amber-400 hover:bg-amber-500/25 border-amber-500/20">
          cancelled
        </Badge>
      );
    case "cancelling":
      return (
        <Badge className="bg-amber-500/15 text-amber-700 dark:text-amber-400 hover:bg-amber-500/25 border-amber-500/20">
          cancelling
        </Badge>
      );
    case "running":
      return (
        <Badge className="bg-blue-500/15 text-blue-700 dark:text-blue-400 hover:bg-blue-500/25 border-blue-500/20">
          {job.stage === "pending" ? "starting" : job.stage.replace(/_/g, " ")}
        </Badge>
      );
    default:
      return <Badge variant="secondary">{job.status}</Badge>;
  }
}

function phaseIndex(stage: string): number {
  if (stage === "resolving_and_fetching") return 1;
  const idx = PROGRESS_PHASES.indexOf(
    stage as (typeof PROGRESS_PHASES)[number],
  );
  return idx;
}

// SSE hook for backfill events
function useBackfillSSE(jobId: string | null, active: boolean): BackfillEvent[] {
  const [events, setEvents] = useState<BackfillEvent[]>([]);

  useEffect(() => {
    if (!jobId || !active) {
      setEvents([]);
      return;
    }

    const basePath = process.env.NEXT_PUBLIC_BASE_PATH || "";
    const es = new EventSource(`${basePath}/admin/backfill/${jobId}/events`, {
      withCredentials: true,
    });

    es.addEventListener("event", (e) => {
      try {
        const event: BackfillEvent = JSON.parse((e as MessageEvent).data);
        setEvents((prev) => [...prev, event]);
      } catch { /* ignore parse errors */ }
    });

    return () => es.close();
  }, [jobId, active]);

  return events;
}

// Batch Bluesky profile resolution hook
function useBlueskyProfiles(dids: string[]): Map<string, BlueskyProfile> {
  const [profiles, setProfiles] = useState<Map<string, BlueskyProfile>>(new Map());
  const resolvedRef = useRef<Set<string>>(new Set());
  const pendingRef = useRef(false);

  useEffect(() => {
    const unresolved = dids.filter((d) => !resolvedRef.current.has(d));
    if (unresolved.length === 0 || pendingRef.current) return;

    pendingRef.current = true;

    const batches: string[][] = [];
    for (let i = 0; i < unresolved.length; i += 25) {
      batches.push(unresolved.slice(i, i + 25));
    }

    // Mark all as resolved immediately to prevent re-fetching
    for (const did of unresolved) {
      resolvedRef.current.add(did);
    }

    Promise.all(
      batches.map(async (batch) => {
        const params = batch.map((d) => `actors=${encodeURIComponent(d)}`).join("&");
        try {
          const resp = await fetch(
            `https://public.api.bsky.app/xrpc/app.bsky.actor.getProfiles?${params}`
          );
          if (!resp.ok) return [];
          const data = await resp.json();
          return (data.profiles || []) as BlueskyProfile[];
        } catch {
          return [];
        }
      })
    ).then((results) => {
      setProfiles((prev) => {
        const newProfiles = new Map(prev);
        for (const batch of results) {
          for (const p of batch) {
            newProfiles.set(p.did, p);
          }
        }
        return newProfiles;
      });
      pendingRef.current = false;
    });
  }, [dids]);

  return profiles;
}

const BSKY_PDS_SUFFIX = ".bsky.network";
const BSKY_PDS_HOSTNAMES = ["bsky.social", "staging.bsky.dev"];
const failedFaviconUrls = new Set<string>();

function isBskyPds(pdsEndpoint: string): boolean {
  try {
    const hostname = new URL(pdsEndpoint).hostname;
    return BSKY_PDS_HOSTNAMES.includes(hostname) || hostname.endsWith(BSKY_PDS_SUFFIX);
  } catch {
    return false;
  }
}

function PdsFavicon({ pdsEndpoint }: { pdsEndpoint: string }) {
  let hostname: string;
  try {
    hostname = new URL(pdsEndpoint).hostname;
  } catch {
    return <PdsPlaceholderIcon />;
  }

  if (isBskyPds(pdsEndpoint)) {
    return (
      <svg viewBox="0 0 360 320" className="size-4 shrink-0" aria-hidden>
        <path
          d="M180 142c-16.3-31.7-60.7-90.8-102-120C38.5-5.9 0 1.4 0 45.6c0 31.7 18.2 266.4 67.8 266.4 45.2 0 60.4-39 112.2-130 51.8 91 67 130 112.2 130C341.8 312 360 77.3 360 45.6 360 1.4 321.5-5.9 282 22c-41.3 29.2-85.7 88.3-102 120Z"
          fill="#1185FE"
        />
      </svg>
    );
  }

  const faviconUrl = `https://twenty-icons.com/${hostname}`;

  if (failedFaviconUrls.has(faviconUrl)) {
    return <PdsPlaceholderIcon />;
  }

  return <PdsFaviconImg url={faviconUrl} />;
}

function PdsFaviconImg({ url }: { url: string }) {
  const [failed, setFailed] = useState(false);

  if (failed) return <PdsPlaceholderIcon />;

  return (
    <img
      src={url}
      alt=""
      className="size-4 shrink-0 rounded"
      onError={() => { failedFaviconUrls.add(url); setFailed(true); }}
    />
  );
}

function PdsPlaceholderIcon() {
  return (
    <svg viewBox="0 0 16 16" className="size-4 shrink-0 text-muted-foreground" fill="none" stroke="currentColor" strokeWidth="1.2" aria-hidden>
      <ellipse cx="8" cy="12" rx="6" ry="2.5" />
      <ellipse cx="8" cy="8" rx="6" ry="2.5" />
      <path d="M2 8v4M14 8v4" />
      <ellipse cx="8" cy="4" rx="6" ry="2.5" />
      <path d="M2 4v4M14 4v4" />
    </svg>
  );
}

function ScrollSentinel({ onVisible }: { onVisible: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  const onVisibleRef = useRef(onVisible);
  onVisibleRef.current = onVisible;

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const observer = new IntersectionObserver(
      ([entry]) => { if (entry.isIntersecting) onVisibleRef.current(); },
      { rootMargin: "100px" },
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  return <div ref={ref} className="h-1" />;
}

function AnimatedNumber({ value }: { value: number }) {
  const targetRef = useRef(value);
  const displayedRef = useRef(value);
  const [displayed, setDisplayed] = useState(value);
  const rafRef = useRef<number>(0);

  targetRef.current = value;

  useEffect(() => {
    cancelAnimationFrame(rafRef.current);

    function tick() {
      const current = displayedRef.current;
      const target = targetRef.current;
      const diff = target - current;

      if (Math.abs(diff) < 0.5) {
        displayedRef.current = target;
        setDisplayed(target);
        return;
      }

      const next = current + diff * 0.06;
      displayedRef.current = next;
      setDisplayed(next);
      rafRef.current = requestAnimationFrame(tick);
    }

    tick();
    return () => cancelAnimationFrame(rafRef.current);
  }, [value]);

  return <>{Math.round(displayed).toLocaleString()}</>;
}

function CompactRepoRow({ did, profile }: {
  did: string;
  profile?: BlueskyProfile;
}) {
  return (
    <div className="flex items-center gap-1.5 px-3 py-1 text-xs">
      <div className="size-4 shrink-0 rounded-full bg-muted overflow-hidden">
        {profile?.avatar && (
          <img src={profile.avatar} alt="" className="size-full object-cover" />
        )}
      </div>
      <span className="truncate">
        {profile?.handle ? `@${profile.handle}` : <span className="font-mono">{did}</span>}
      </span>
    </div>
  );
}

function ProfileRow({ did, profile, suffix }: {
  did: string;
  profile?: BlueskyProfile;
  suffix?: React.ReactNode;
}) {
  return (
    <div className="flex items-center gap-2 px-3 py-1.5 text-sm">
      <div className="size-6 shrink-0 rounded-full bg-muted overflow-hidden">
        {profile?.avatar && (
          <img src={profile.avatar} alt="" className="size-full object-cover" />
        )}
      </div>
      <div className="flex-1 min-w-0">
        <p className="truncate text-sm">
          {profile?.displayName || profile?.handle || did}
        </p>
        {profile?.handle && (
          <p className="truncate text-xs text-muted-foreground">@{profile.handle}</p>
        )}
        {!profile?.handle && (
          <p className="truncate text-xs text-muted-foreground font-mono">{did}</p>
        )}
      </div>
      {suffix && (
        <span className="shrink-0 text-xs text-muted-foreground tabular-nums">{suffix}</span>
      )}
    </div>
  );
}

export default function BackfillPage() {
  const { hasPermission } = useCurrentUser();
  const [jobs, setJobs] = useState<BackfillJob[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [selectedJobId, setSelectedJobId] = useState<string | null>(null);

  const load = useCallback(() => {
    getBackfillJobs()
      .then(setJobs)
      .catch((e) => setError(e.message));
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  useEffect(() => {
    const interval = setInterval(load, 5000);
    return () => clearInterval(interval);
  }, [load]);

  const selectedJob = jobs.find((j) => j.id === selectedJobId) ?? null;
  const canFlush = hasPermission("backfill:create");

  return (
    <>
      <SiteHeader title="Backfill" />
      <div className="flex flex-1 flex-col gap-4 p-4 md:p-6">
        {error && <p className="text-destructive text-sm">{error}</p>}

        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold">Backfill Jobs</h2>
          <div className="flex items-center gap-2">
            {canFlush && (
              <AlertDialog>
                <AlertDialogTrigger asChild>
                  <Button variant="outline" size="sm">Clear all details</Button>
                </AlertDialogTrigger>
                <AlertDialogContent>
                  <AlertDialogHeader>
                    <AlertDialogTitle>Clear all job details?</AlertDialogTitle>
                    <AlertDialogDescription>
                      This will permanently delete per-repo detail data for all backfill jobs.
                    </AlertDialogDescription>
                  </AlertDialogHeader>
                  <AlertDialogFooter>
                    <AlertDialogCancel>Cancel</AlertDialogCancel>
                    <AlertDialogAction onClick={async () => {
                      await flushAllBackfillDetails();
                    }}>Clear</AlertDialogAction>
                  </AlertDialogFooter>
                </AlertDialogContent>
              </AlertDialog>
            )}
            {hasPermission("backfill:create") && (
              <CreateDialog onSuccess={load} />
            )}
          </div>
        </div>

        <div className="rounded-lg border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>ID</TableHead>
                <TableHead>Collection</TableHead>
                <TableHead>DID</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>Started</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {jobs.length === 0 && (
                <TableRow>
                  <TableCell
                    colSpan={5}
                    className="text-muted-foreground text-center"
                  >
                    No backfill jobs yet.
                  </TableCell>
                </TableRow>
              )}
              {jobs.map((job) => (
                <TableRow
                  key={job.id}
                  className="cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  tabIndex={0}
                  role="button"
                  onClick={() => setSelectedJobId(job.id)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      setSelectedJobId(job.id);
                    }
                  }}
                >
                  <TableCell className="font-mono text-xs">
                    {job.id.slice(0, 8)}
                  </TableCell>
                  <TableCell className="font-mono text-sm">
                    {job.collection ?? "All"}
                  </TableCell>
                  <TableCell className="font-mono text-sm">
                    {job.did ?? "All"}
                  </TableCell>
                  <TableCell>{statusBadge(job)}</TableCell>
                  <TableCell>
                    {job.started_at
                      ? new Date(job.started_at).toLocaleString()
                      : "--"}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>

        <Sheet
          open={selectedJob != null}
          onOpenChange={(open) => {
            if (!open) setSelectedJobId(null);
          }}
        >
          <SheetContent className="sm:max-w-xl overflow-hidden flex flex-col">
            {selectedJob && (
              <JobDetail
                job={selectedJob}
                canCancel={hasPermission("backfill:create")}
                canFlush={canFlush}
                onCancel={async () => {
                  await cancelBackfillJob(selectedJob.id);
                  load();
                }}
              />
            )}
          </SheetContent>
        </Sheet>
      </div>
    </>
  );
}

function JobDetail({
  job,
  canCancel,
  canFlush,
  onCancel,
}: {
  job: BackfillJob;
  canCancel: boolean;
  canFlush: boolean;
  onCancel: () => Promise<void>;
}) {
  const [cancelling, setCancelling] = useState(false);
  const current = phaseIndex(job.stage);
  const allDone = job.status === "completed";
  const isActive = job.status === "running" || job.status === "cancelling";

  // Detail data state
  const [discoveredRepos, setDiscoveredRepos] = useState<BackfillRepoEntry[]>([]);
  const [discoveredCursor, setDiscoveredCursor] = useState<string | null>(null);
  const [discoveredLoaded, setDiscoveredLoaded] = useState(false);

  const [pdsSummary, setPdsSummary] = useState<PdsSummaryEntry[]>([]);
  const [pdsLoaded, setPdsLoaded] = useState(false);

  const [fetchedRepos, setFetchedRepos] = useState<BackfillRepoEntry[]>([]);
  const [fetchedCursor, setFetchedCursor] = useState<string | null>(null);
  const [fetchedLoaded, setFetchedLoaded] = useState(false);

  // SSE events for active jobs
  const sseEvents = useBackfillSSE(job.id, isActive);

  function hasReached(phase: (typeof PROGRESS_PHASES)[number]): boolean {
    if (allDone) return true;
    if (job.stage === "resolving_and_fetching") {
      return phase === "discovering_repos" || phase === "resolving_pds" || phase === "fetching_records";
    }
    return current >= phaseIndex(phase);
  }

  async function handleCancel() {
    setCancelling(true);
    try {
      await onCancel();
    } finally {
      setCancelling(false);
    }
  }

  // Auto-load detail data when phases are reached
  const discoveredReached = hasReached("discovering_repos");
  const pdsReached = hasReached("resolving_pds");
  const fetchedReached = hasReached("fetching_records") || job.stage === "resolving_and_fetching";

  useEffect(() => {
    if (discoveredReached && !discoveredLoaded) {
      getBackfillRepos(job.id, { phase: "discovered", limit: 50 })
        .then((resp) => { setDiscoveredRepos(resp.repos); setDiscoveredCursor(resp.cursor); setDiscoveredLoaded(true); })
        .catch(() => {});
    }
  }, [job.id, discoveredReached, discoveredLoaded]);

  useEffect(() => {
    if (pdsReached && !pdsLoaded) {
      getBackfillPdsSummary(job.id)
        .then((resp) => { setPdsSummary(resp.pds_endpoints); setPdsLoaded(true); })
        .catch(() => {});
    }
  }, [job.id, pdsReached, pdsLoaded]);

  useEffect(() => {
    if (fetchedReached && !fetchedLoaded) {
      getBackfillRepos(job.id, { phase: "fetched", limit: 50 })
        .then((resp) => { setFetchedRepos(resp.repos); setFetchedCursor(resp.cursor); setFetchedLoaded(true); })
        .catch(() => {});
    }
  }, [job.id, fetchedReached, fetchedLoaded]);

  const loadMoreDiscovered = useCallback(async () => {
    if (!discoveredCursor) return;
    try {
      const resp = await getBackfillRepos(job.id, { phase: "discovered", cursor: discoveredCursor, limit: 50 });
      setDiscoveredRepos((prev) => [...prev, ...resp.repos]);
      setDiscoveredCursor(resp.cursor);
    } catch { /* ignore */ }
  }, [job.id, discoveredCursor]);

  const loadMoreFetched = useCallback(async () => {
    if (!fetchedCursor) return;
    try {
      const resp = await getBackfillRepos(job.id, { phase: "fetched", cursor: fetchedCursor, limit: 50 });
      setFetchedRepos((prev) => [...prev, ...resp.repos]);
      setFetchedCursor(resp.cursor);
    } catch { /* ignore */ }
  }, [job.id, fetchedCursor]);

  // Process SSE events
  useEffect(() => {
    for (const event of sseEvents) {
      if (event.type === "repo_discovered" && event.did) {
        setDiscoveredRepos((prev) => {
          if (prev.some((r) => r.did === event.did)) return prev;
          return [{ did: event.did!, pds_endpoint: null, status: "pending", records_fetched: 0 }, ...prev];
        });
      }
      if (event.type === "repo_resolved" && event.did && event.pds_endpoint) {
        setDiscoveredRepos((prev) =>
          prev.map((r) => r.did === event.did ? { ...r, pds_endpoint: event.pds_endpoint! } : r)
        );
        setPdsSummary((prev) => {
          const idx = prev.findIndex((p) => p.pds_endpoint === event.pds_endpoint);
          if (idx >= 0) {
            const updated = [...prev];
            updated[idx] = { ...updated[idx], total_repos: updated[idx].total_repos + 1 };
            return updated;
          }
          return [...prev, { pds_endpoint: event.pds_endpoint!, total_repos: 1, completed_repos: 0, total_records: 0 }];
        });
      }
      if (event.type === "repo_fetched" && event.did) {
        setFetchedRepos((prev) => {
          if (prev.some((r) => r.did === event.did)) return prev;
          return [{ did: event.did!, pds_endpoint: event.pds_endpoint ?? null, status: "completed", records_fetched: event.records_fetched ?? 0 }, ...prev];
        });
        setPdsSummary((prev) => {
          if (!event.pds_endpoint) return prev;
          return prev.map((p) => p.pds_endpoint === event.pds_endpoint
            ? { ...p, completed_repos: p.completed_repos + 1, total_records: p.total_records + (event.records_fetched ?? 0) }
            : p
          );
        });
      }
    }
  }, [sseEvents]);

  // Collect all visible DIDs for profile resolution
  const allVisibleDids = useMemo(() => {
    const dids = new Set<string>();
    for (const r of discoveredRepos) dids.add(r.did);
    for (const r of fetchedRepos) dids.add(r.did);
    return Array.from(dids);
  }, [discoveredRepos, fetchedRepos]);

  const profiles = useBlueskyProfiles(allVisibleDids);

  return (
    <>
      <SheetHeader>
        <SheetTitle className="flex items-center gap-2">
          <span className="font-mono text-sm">Backfill Details</span>
        </SheetTitle>
      </SheetHeader>
      <div className="flex-1 min-h-0 overflow-y-auto px-4 flex flex-col gap-4">
        <div className="grid grid-cols-2 gap-4 text-sm">
          <div className="col-span-2">
            <span className="text-muted-foreground">Job ID</span>
            <p className="font-mono text-xs break-all">{job.id}</p>
          </div>
          <div>
            <span className="text-muted-foreground">Collection</span>
            <p className="font-mono text-xs">{job.collection ?? "All"}</p>
          </div>
          <div>
            <span className="text-muted-foreground">DID</span>
            <p className="font-mono text-xs break-all">{job.did ?? "All"}</p>
          </div>
          <div>
            <span className="text-muted-foreground">Created</span>
            <p className="text-xs">
              {new Date(job.created_at).toLocaleString()}
            </p>
          </div>
          <div>
            <span className="text-muted-foreground">Started</span>
            <p className="text-xs">
              {job.started_at
                ? new Date(job.started_at).toLocaleString()
                : "--"}
            </p>
          </div>
          {job.completed_at && (
            <div>
              <span className="text-muted-foreground">Completed</span>
              <p className="text-xs">
                {new Date(job.completed_at).toLocaleString()}
              </p>
            </div>
          )}
        </div>

        {job.error && (
          <div>
            <span className="text-muted-foreground text-sm">Error</span>
            <div className="bg-destructive/10 text-destructive mt-1 rounded-md p-3 font-mono text-xs whitespace-pre-wrap">
              {job.error}
            </div>
          </div>
        )}

        <div>
          <span className="text-muted-foreground text-sm">Progress</span>
          <div className="mt-1 rounded-md border divide-y">
            <ProgressRow
              label="Discovering repos"
              active={isActive && job.stage === "discovering_repos"}
              reached={hasReached("discovering_repos")}
              value={job.total_repos != null ? <AnimatedNumber value={job.total_repos} /> : undefined}
              suffix="repos found"
              loading={discoveredReached && !discoveredLoaded}
            >
              {discoveredRepos.length > 0 ? (
                <div>
                  {discoveredRepos.map((repo) => (
                    <CompactRepoRow
                      key={repo.did}
                      did={repo.did}
                      profile={profiles.get(repo.did)}
                    />
                  ))}
                  {discoveredCursor && (
                    <ScrollSentinel onVisible={loadMoreDiscovered} />
                  )}
                </div>
              ) : discoveredLoaded ? (
                <p className="py-3 text-center text-xs text-muted-foreground">No repos discovered yet.</p>
              ) : null}
            </ProgressRow>
            <ProgressRow
              label="Resolving PDS"
              active={
                isActive &&
                (job.stage === "resolving_pds" ||
                  job.stage === "resolving_and_fetching")
              }
              reached={hasReached("resolving_pds")}
              value={
                hasReached("resolving_pds")
                  ? <><AnimatedNumber value={job.resolved_repos ?? job.processed_repos ?? 0} /> / <AnimatedNumber value={job.total_repos ?? 0} /></>
                  : undefined
              }
              suffix="resolved"
              loading={pdsReached && !pdsLoaded}
            >
              {pdsSummary.length > 0 ? (
                <div className="divide-y text-sm">
                  {pdsSummary
                    .sort((a, b) => b.total_repos - a.total_repos)
                    .map((pds) => (
                      <div key={pds.pds_endpoint} className="flex items-center gap-2 px-3 py-1.5">
                        <PdsFavicon pdsEndpoint={pds.pds_endpoint} />
                        <span className="flex-1 truncate font-mono text-xs">{new URL(pds.pds_endpoint).hostname}</span>
                        <span className="text-xs text-muted-foreground tabular-nums">
                          <AnimatedNumber value={pds.completed_repos} />/<AnimatedNumber value={pds.total_repos} /> repos · <AnimatedNumber value={pds.total_records} /> records
                        </span>
                      </div>
                    ))}
                </div>
              ) : pdsLoaded ? (
                <p className="py-3 text-center text-xs text-muted-foreground">No PDS data yet.</p>
              ) : null}
            </ProgressRow>
            <ProgressRow
              label="Fetching records"
              active={
                isActive &&
                (job.stage === "fetching_records" ||
                  job.stage === "resolving_and_fetching")
              }
              reached={hasReached("fetching_records")}
              value={
                hasReached("fetching_records") ||
                job.stage === "resolving_and_fetching"
                  ? <><AnimatedNumber value={job.processed_repos ?? 0} /> / <AnimatedNumber value={job.total_repos ?? 0} /> repos</>
                  : undefined
              }
              suffix={
                hasReached("fetching_records") ||
                job.stage === "resolving_and_fetching"
                  ? <><AnimatedNumber value={job.total_records ?? 0} /> records</>
                  : undefined
              }
              loading={fetchedReached && !fetchedLoaded}
            >
              {fetchedRepos.filter((r) => r.records_fetched > 0).length > 0 ? (
                <div className="divide-y">
                  {fetchedRepos.filter((r) => r.records_fetched > 0).map((repo) => (
                    <ProfileRow
                      key={repo.did}
                      did={repo.did}
                      profile={profiles.get(repo.did)}
                      suffix={<span className="flex items-center gap-1"><CheckCircle2 className="size-3 text-emerald-500" /><AnimatedNumber value={repo.records_fetched} /> records</span>}
                    />
                  ))}
                  {fetchedCursor && (
                    <ScrollSentinel onVisible={loadMoreFetched} />
                  )}
                </div>
              ) : fetchedLoaded ? (
                <p className="py-3 text-center text-xs text-muted-foreground">No repos fetched yet.</p>
              ) : null}
            </ProgressRow>
          </div>
        </div>
      </div>
      <SheetFooter className="border-t flex-row justify-end gap-2">
        {canFlush && !isActive && (
          <AlertDialog>
            <AlertDialogTrigger asChild>
              <Button variant="outline" size="sm">Clear details</Button>
            </AlertDialogTrigger>
            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle>Clear job details?</AlertDialogTitle>
                <AlertDialogDescription>
                  This will permanently delete per-repo detail data for this backfill job.
                </AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel>Cancel</AlertDialogCancel>
                <AlertDialogAction onClick={async () => {
                  await flushBackfillDetails(job.id);
                  setDiscoveredRepos([]);
                  setDiscoveredCursor(null);
                  setDiscoveredLoaded(false);
                  setPdsSummary([]);
                  setPdsLoaded(false);
                  setFetchedRepos([]);
                  setFetchedCursor(null);
                  setFetchedLoaded(false);
                }}>Clear</AlertDialogAction>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>
        )}
        {canCancel && isActive && (
          <Button
            variant="destructive"
            size="sm"
            disabled={cancelling || job.status === "cancelling"}
            onClick={handleCancel}
          >
            {job.status === "cancelling" ? "Cancelling…" : "Cancel Job"}
          </Button>
        )}
      </SheetFooter>
    </>
  );
}

function ProgressRow({
  label,
  active,
  reached,
  value,
  suffix,
  loading,
  children,
}: {
  label: string;
  active: boolean;
  reached: boolean;
  value?: React.ReactNode;
  suffix?: React.ReactNode;
  loading?: boolean;
  children?: React.ReactNode;
}) {
  const [open, setOpen] = useState(false);
  const done = reached && !active;
  const expandable = reached;

  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <CollapsibleTrigger asChild disabled={!expandable}>
        <div
          className={`flex items-center gap-2 px-3 py-2 text-sm ${
            expandable ? "cursor-pointer hover:bg-accent/50" : ""
          } ${
            active ? "bg-blue-500/5" : reached ? "" : "text-muted-foreground opacity-50"
          }`}
        >
          <span className="shrink-0">
            {active ? (
              <Loader2 className="size-4 animate-spin text-blue-500" />
            ) : done ? (
              <CheckCircle2 className="size-4 text-emerald-500" />
            ) : (
              <Circle className="size-4" />
            )}
          </span>
          <span className={`flex-1 ${active ? "font-medium" : ""}`}>{label}</span>
          {reached && value && (
            <span className="tabular-nums text-xs text-muted-foreground">
              {value}
              {suffix && <> · {suffix}</>}
            </span>
          )}
          {expandable && (
            <ChevronRight className={`size-4 text-muted-foreground transition-transform ${open ? "rotate-90" : ""}`} />
          )}
        </div>
      </CollapsibleTrigger>
      {expandable && (
        <CollapsibleContent>
          <div className="border-t bg-muted/30 max-h-64 overflow-y-auto">
            {loading ? (
              <div className="flex items-center justify-center py-4">
                <Loader2 className="size-4 animate-spin text-muted-foreground" />
              </div>
            ) : children}
          </div>
        </CollapsibleContent>
      )}
    </Collapsible>
  );
}

function CreateDialog({ onSuccess }: { onSuccess: () => void }) {
  const [collection, setCollection] = useState<string | null>(null);
  const [did, setDid] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [open, setOpen] = useState(false);
  const [recordLexicons, setRecordLexicons] = useState<string[]>([]);

  useEffect(() => {
    if (open) {
      getLexicons()
        .then((lexicons) =>
          setRecordLexicons(
            lexicons
              .filter((l) => l.lexicon_type === "record")
              .map((l) => l.id)
              .sort(),
          ),
        )
        .catch(() => {});
    }
  }, [open]);

  async function handleCreate() {
    setError(null);
    try {
      await createBackfillJob({
        collection: collection || undefined,
        did: did || undefined,
      });
      setCollection(null);
      setDid("");
      setOpen(false);
      onSuccess();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  return (
    <ResponsiveDialog open={open} onOpenChange={setOpen}>
      <ResponsiveDialogTrigger asChild>
        <Button>Create Backfill Job</Button>
      </ResponsiveDialogTrigger>
      <ResponsiveDialogContent
        onInteractOutside={(e) => {
          const target = e.target as HTMLElement;
          if (
            target.closest(
              "[data-slot='combobox-item'], [data-slot='combobox-content']",
            )
          ) {
            e.preventDefault();
          }
        }}
      >
        <ResponsiveDialogHeader>
          <ResponsiveDialogTitle>Create Backfill Job</ResponsiveDialogTitle>
          <ResponsiveDialogDescription>
            Start a backfill for a collection or specific DID. Leave both empty
            to backfill all collections.
          </ResponsiveDialogDescription>
        </ResponsiveDialogHeader>
        <div className="flex flex-col gap-4">
          {error && <p className="text-destructive text-sm">{error}</p>}
          <div className="flex flex-col gap-2">
            <Label>Collection (optional)</Label>
            <Combobox
              value={collection}
              onValueChange={setCollection}
              items={recordLexicons}
            >
              <ComboboxInput
                placeholder="Select or type a collection..."
                showClear
              />
              <ComboboxContent>
                <ComboboxEmpty>No matching lexicons.</ComboboxEmpty>
                <ComboboxList>
                  {(item: string) => (
                    <ComboboxItem key={item} value={item}>
                      {item}
                    </ComboboxItem>
                  )}
                </ComboboxList>
              </ComboboxContent>
            </Combobox>
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="bf-did">DID (optional)</Label>
            <Input
              id="bf-did"
              value={did}
              onChange={(e) => setDid(e.target.value)}
              placeholder="did:plc:..."
            />
          </div>
        </div>
        <ResponsiveDialogFooter>
          <ResponsiveDialogClose asChild>
            <Button variant="outline">Cancel</Button>
          </ResponsiveDialogClose>
          <Button onClick={handleCreate}>Create</Button>
        </ResponsiveDialogFooter>
      </ResponsiveDialogContent>
    </ResponsiveDialog>
  );
}

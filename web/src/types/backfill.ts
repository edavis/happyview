export interface BackfillJob {
  id: string
  collection: string | null
  did: string | null
  status: string
  stage: string
  total_repos: number | null
  resolved_repos: number | null
  processed_repos: number | null
  total_records: number | null
  error: string | null
  started_at: string | null
  completed_at: string | null
  created_at: string
}

export interface BackfillRepoEntry {
  did: string
  pds_endpoint: string | null
  status: string
  records_fetched: number
}

export interface BackfillReposResponse {
  repos: BackfillRepoEntry[]
  cursor: string | null
}

export interface PdsSummaryEntry {
  pds_endpoint: string
  total_repos: number
  completed_repos: number
  total_records: number
}

export interface PdsSummaryResponse {
  pds_endpoints: PdsSummaryEntry[]
}

export interface BackfillEvent {
  type: string
  job_id: string
  did?: string
  pds_endpoint?: string
  records_fetched?: number
  total_repos?: number | null
  resolved_repos?: number | null
  processed_repos?: number | null
  total_records?: number | null
  stage?: string
  status?: string
  error?: string | null
}

export interface BlueskyProfile {
  did: string
  handle: string
  displayName?: string
  avatar?: string
}

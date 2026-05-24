interface BackfillEvent {
  type: string;
  did?: string;
  pds_endpoint?: string;
  records_fetched?: number;
}

interface ConnectMessage {
  type: "connect";
  jobId: string;
  baseUrl: string;
}

interface DisconnectMessage {
  type: "disconnect";
}

type WorkerMessage = ConnectMessage | DisconnectMessage;

let eventSource: EventSource | null = null;
let buffer: BackfillEvent[] = [];
let flushTimer: ReturnType<typeof setInterval> | null = null;

const FLUSH_INTERVAL = 500;

function flush() {
  if (buffer.length === 0) return;
  const batch = buffer;
  buffer = [];
  self.postMessage({ type: "batch", events: batch });
}

function connect(jobId: string, baseUrl: string) {
  disconnect();

  eventSource = new EventSource(`${baseUrl}/admin/backfill/${jobId}/events`, {
    withCredentials: true,
  });

  eventSource.addEventListener("event", (e) => {
    try {
      const event: BackfillEvent = JSON.parse((e as MessageEvent).data);
      buffer.push(event);
    } catch {
      // ignore parse errors
    }
  });

  eventSource.addEventListener("error", () => {
    self.postMessage({ type: "error" });
  });

  flushTimer = setInterval(flush, FLUSH_INTERVAL);
}

function disconnect() {
  if (eventSource) {
    eventSource.close();
    eventSource = null;
  }
  if (flushTimer) {
    clearInterval(flushTimer);
    flushTimer = null;
  }
  flush();
}

self.addEventListener("message", (e: MessageEvent<WorkerMessage>) => {
  const msg = e.data;
  if (msg.type === "connect") {
    connect(msg.jobId, msg.baseUrl);
  } else if (msg.type === "disconnect") {
    disconnect();
  }
});

// Holds the most recent camera capture between the capture → preview → send
// screens. Kept in memory rather than in route params because the base64
// payload is several megabytes.
export interface PendingCapture {
  uri: string;
  base64: string;
}

let pending: PendingCapture | null = null;

export function setPendingCapture(capture: PendingCapture) {
  pending = capture;
}

export function getPendingCapture(): PendingCapture | null {
  return pending;
}

export function clearPendingCapture() {
  pending = null;
}

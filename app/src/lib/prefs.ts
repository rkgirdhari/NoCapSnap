// Per-device UI preferences. Browser storage can be unavailable or cleared, so
// every read falls back to a default and every write is best-effort.

export type CameraRoute = "viewfinder" | "phone";

const CAMERA_KEY = "capsnap.cameraRoute";

export function cameraRoute(): CameraRoute {
  try {
    return localStorage.getItem(CAMERA_KEY) === "phone" ? "phone" : "viewfinder";
  } catch {
    return "viewfinder";
  }
}

export function setCameraRoute(route: CameraRoute): void {
  try {
    localStorage.setItem(CAMERA_KEY, route);
  } catch {
    // Not persisted; the default applies next launch.
  }
}

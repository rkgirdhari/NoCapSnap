class Connectivity {
  online = $state(typeof navigator === "undefined" ? true : navigator.onLine);

  constructor() {
    if (typeof window === "undefined") return;
    window.addEventListener("online", () => (this.online = true));
    window.addEventListener("offline", () => (this.online = false));
  }
}

export const connectivity = new Connectivity();

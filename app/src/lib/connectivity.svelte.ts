// The header's Online/Offline status. It only shows a status: syncing never waits on it.
//
// On Android the app asks the phone (NetworkStatus.kt): online means a Wi-Fi or mobile network that Android
// has checked reaches the internet. The WebView's navigator.onLine only says some network is up, and kept
// saying online in airplane mode while a VPN stayed up (D9). Elsewhere (Windows, the browser preview),
// navigator.onLine and its events.
type NativeNetwork = { isOnline(): boolean };

class Connectivity {
  online = $state(true);

  constructor() {
    if (typeof window === "undefined") return;
    const native = (window as unknown as { CapSnapNetwork?: NativeNetwork }).CapSnapNetwork;
    if (native) {
      this.online = native.isOnline();
      window.addEventListener("capsnap-network", (e) => (this.online = (e as CustomEvent<boolean>).detail === true));
      // A change while the app was in the background may have gone to a page that wasn't listening.
      document.addEventListener("visibilitychange", () => {
        if (document.visibilityState === "visible") this.online = native.isOnline();
      });
      return;
    }
    this.online = navigator.onLine;
    window.addEventListener("online", () => (this.online = true));
    window.addEventListener("offline", () => (this.online = false));
  }
}

export const connectivity = new Connectivity();

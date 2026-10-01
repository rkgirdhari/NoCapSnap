/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** "1" builds the in-memory browser preview into the bundle (design review only). */
  readonly VITE_CAPSNAP_PREVIEW?: string;
}

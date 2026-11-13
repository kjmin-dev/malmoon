interface ImportMetaEnv {
  /** Base URL of the gateway. Defaults to ws://127.0.0.1:8787. */
  readonly VITE_GATEWAY_URL?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

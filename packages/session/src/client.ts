import type { CaptionUpdate } from "./generated/CaptionUpdate.ts";
import type { ClientKind } from "./generated/ClientKind.ts";
import type { ClientMessage } from "./generated/ClientMessage.ts";
import { PROTOCOL_VERSION, SESSION_PATH } from "./generated/constants.ts";
import type { ServerMessage } from "./generated/ServerMessage.ts";

export type SessionEvent =
  | { type: "connecting" }
  | { type: "ready"; sessionId: string }
  | { type: "caption"; caption: CaptionUpdate }
  | { type: "disconnected"; reason: string };

export type SessionOptions = {
  /** The gateway's base URL, for example `ws://127.0.0.1:8787`. */
  url: string;
  client: ClientKind;
  /**
   * The session token of the signed-in user, for React Native. Browsers cannot set headers on a
   * WebSocket and do not need to: they send the session cookie.
   */
  token?: string | undefined;
  onEvent: (event: SessionEvent) => void;
};

export type Session = {
  /** Closes the connection and stops reconnecting. */
  close: () => void;
};

const RETRY_MIN_MS = 1_000;
const RETRY_MAX_MS = 10_000;

/**
 * Opens a session with the gateway and keeps it open, reconnecting with backoff until `close` is
 * called. Uses the global `WebSocket`, which browsers and React Native both provide.
 */
export function connectSession({ url, client, token, onEvent }: SessionOptions): Session {
  const endpoint = url.replace(/\/+$/, "") + SESSION_PATH;
  let socket: WebSocket | undefined;
  let retryTimer: ReturnType<typeof setTimeout> | undefined;
  let retryMs = RETRY_MIN_MS;
  let closed = false;

  const connect = () => {
    onEvent({ type: "connecting" });
    const current = token ? openWithToken(endpoint, token) : new WebSocket(endpoint);
    socket = current;
    // The gateway's own explanation, when it sends one before closing.
    let refusal: string | undefined;

    current.onopen = () => {
      const hello: ClientMessage = { type: "hello", protocol_version: PROTOCOL_VERSION, client };
      current.send(JSON.stringify(hello));
    };

    current.onmessage = (event) => {
      if (typeof event.data !== "string") return;
      const message = JSON.parse(event.data) as ServerMessage;
      switch (message.type) {
        case "ready":
          retryMs = RETRY_MIN_MS;
          onEvent({ type: "ready", sessionId: message.session_id });
          break;
        case "caption":
          onEvent({ type: "caption", caption: message.caption });
          break;
        case "error":
          refusal = `the gateway refused the session (${message.code}): ${message.message}`;
          break;
      }
    };

    current.onclose = () => {
      if (closed || socket !== current) return;
      onEvent({ type: "disconnected", reason: refusal ?? `could not reach ${endpoint}` });
      retryTimer = setTimeout(connect, retryMs);
      retryMs = Math.min(retryMs * 2, RETRY_MAX_MS);
    };
  };

  connect();
  return {
    close: () => {
      closed = true;
      clearTimeout(retryTimer);
      socket?.close();
    },
  };
}

/** React Native's WebSocket takes request headers as a third argument. The standard has none. */
function openWithToken(endpoint: string, token: string): WebSocket {
  const NativeWebSocket = WebSocket as unknown as new (
    url: string,
    protocols: undefined,
    options: { headers: Record<string, string> },
  ) => WebSocket;
  return new NativeWebSocket(endpoint, undefined, {
    headers: { Authorization: `Bearer ${token}` },
  });
}

import { useEffect, useState } from "react";
import { applyCaption } from "./captions.ts";
import { connectSession } from "./client.ts";
import type { CaptionUpdate } from "./generated/CaptionUpdate.ts";
import type { ClientKind } from "./generated/ClientKind.ts";

export type SessionStatus =
  | { state: "connecting" }
  | { state: "live" }
  | { state: "offline"; reason: string };

export type LiveSession = {
  status: SessionStatus;
  /** Segments in speaking order, oldest first. */
  segments: readonly CaptionUpdate[];
};

/** Settled segments kept on screen. Older ones scroll away. */
const VISIBLE_SEGMENTS = 3;

/**
 * Keeps a gateway session open while the component is mounted and tracks its captions. The
 * gateway only accepts signed-in users: mount this once there is one, and pass the session token
 * in React Native.
 */
export function useSession(url: string, client: ClientKind, token?: string): LiveSession {
  const [status, setStatus] = useState<SessionStatus>({ state: "connecting" });
  const [segments, setSegments] = useState<readonly CaptionUpdate[]>([]);

  useEffect(() => {
    const session = connectSession({
      url,
      client,
      token,
      onEvent: (event) => {
        switch (event.type) {
          case "connecting":
            setStatus({ state: "connecting" });
            break;
          case "ready":
            setStatus({ state: "live" });
            // Segment ids restart with every session.
            setSegments([]);
            break;
          case "caption":
            setSegments((current) => applyCaption(current, event.caption, VISIBLE_SEGMENTS));
            break;
          case "disconnected":
            setStatus({ state: "offline", reason: event.reason });
            break;
        }
      },
    });
    return session.close;
  }, [url, client, token]);

  return { status, segments };
}

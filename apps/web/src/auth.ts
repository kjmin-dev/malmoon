import {
  type Account,
  AuthError,
  createAuthClient,
  type DesktopHandoff,
  errorMessage,
  type PendingOAuth,
  PROVIDER_NAME,
  type SocialProvider,
} from "@malmoon/auth";

// `localhost`, not 127.0.0.1: the session cookie is only sent between the web app and the
// gateway when the browser sees them as the same site, and it compares host names.
export const GATEWAY_URL: string = import.meta.env.VITE_GATEWAY_URL ?? "http://localhost:8787";
export const SESSION_URL = GATEWAY_URL.replace(/^http/, "ws");

export const auth = createAuthClient({ baseUrl: GATEWAY_URL, client: "web" });

const CALLBACK_PATH = "/auth/callback";
const DESKTOP_PATH = "/auth/desktop";
const PENDING_KEY = "malmoon.oauth";
const HANDOFF_KEY = "malmoon.handoff";

/** Leaves the page for the provider. The answer arrives on `/auth/callback`. */
export async function beginOAuth(provider: SocialProvider, { link = false } = {}): Promise<void> {
  const { authorizationUrl, pending } = await auth.startOAuth(provider, {
    redirectUri: location.origin + CALLBACK_PATH,
    link,
  });
  // sessionStorage is per tab, so the answer is only accepted in the tab that asked.
  sessionStorage.setItem(PENDING_KEY, JSON.stringify(pending));
  location.assign(authorizationUrl);
}

/**
 * The desktop app opened this page to be signed in. Kept in sessionStorage because signing in
 * here first may take the tab to a provider and back.
 */
function desktopHandoff(): DesktopHandoff | undefined {
  if (location.pathname === DESKTOP_PATH) {
    const params = new URLSearchParams(location.search);
    const redirectUri = params.get("redirect_uri");
    const codeChallenge = params.get("code_challenge");
    history.replaceState(null, "", "/");
    if (redirectUri && codeChallenge) {
      const handoff = { redirectUri, codeChallenge, state: params.get("state") ?? undefined };
      sessionStorage.setItem(HANDOFF_KEY, JSON.stringify(handoff));
    }
  }
  const stored = sessionStorage.getItem(HANDOFF_KEY);
  return stored ? (JSON.parse(stored) as DesktopHandoff) : undefined;
}

export function forgetDesktopHandoff(): void {
  sessionStorage.removeItem(HANDOFF_KEY);
}

export type Boot = {
  account: Account | null;
  /** Set when the desktop app is waiting for this browser to sign it in. */
  handoff?: DesktopHandoff | undefined;
  /** What to tell the person about the sign-in they just came back from. */
  notice?: { tone: "success" | "danger"; text: string } | undefined;
};

async function boot(): Promise<Boot> {
  const handoff = desktopHandoff();
  let notice: Boot["notice"];
  if (location.pathname === CALLBACK_PATH) {
    const callbackUrl = location.href;
    const stored = sessionStorage.getItem(PENDING_KEY);
    sessionStorage.removeItem(PENDING_KEY);
    // The login code must not stay in the address bar or the history.
    history.replaceState(null, "", "/");
    try {
      if (!stored) throw new AuthError("flow_expired", "no sign-in was started in this tab");
      const result = await auth.finishOAuth(JSON.parse(stored) as PendingOAuth, callbackUrl);
      if (result.type === "linked") {
        notice = {
          tone: "success",
          text: `${PROVIDER_NAME[result.provider]} 로그인을 연결했습니다.`,
        };
      }
    } catch (error) {
      notice = { tone: "danger", text: errorMessage(error) };
    }
  }
  try {
    return { account: await auth.account(), handoff, notice };
  } catch (error) {
    return {
      account: null,
      handoff,
      notice: notice ?? { tone: "danger", text: errorMessage(error) },
    };
  }
}

/**
 * Runs once per page load, outside React: a login code works once, and React's development mode
 * runs effects twice.
 */
export const booted: Promise<Boot> = boot();

import { type CryptoAdapter, codeChallenge, randomSecret, webCrypto } from "./pkce.ts";

export type ClientKind = "desktop" | "web" | "mobile";
export type Provider = "email" | "google" | "apple" | "kakao" | "line";
export type SocialProvider = Exclude<Provider, "email">;

export type User = { id: string; display_name: string | null; created_at: string };

export type Identity = {
  id: string;
  provider: Provider;
  email: string | null;
  email_verified: boolean;
  display_name: string | null;
  created_at: string;
  last_login_at: string;
};

/** A user and every way they can sign in. */
export type Account = { user: User; identities: Identity[] };

export type SignedIn = Account & {
  /**
   * The new session. `token` is null in the web app, where the session lives in a cookie that
   * scripts cannot read. Null altogether when the request linked a sign-in method instead.
   */
  session: { token: string | null; expires_at: string } | null;
};

/** The codes the account API answers with. Clients switch on these, not on messages. */
export type AuthErrorCode =
  | "invalid_request"
  | "unauthorized"
  | "forbidden_origin"
  | "provider_unavailable"
  | "flow_expired"
  | "invalid_code"
  | "too_many_attempts"
  | "rate_limited"
  | "email_in_use"
  | "identity_in_use"
  | "last_identity"
  | "cancelled"
  | "provider_error"
  | "internal"
  /** The gateway could not be reached, or answered with something else entirely. */
  | "network";

export class AuthError extends Error {
  readonly code: AuthErrorCode;
  /** Seconds to wait, on `rate_limited`. */
  readonly retryAfter: number | undefined;

  constructor(code: AuthErrorCode, message: string, retryAfter?: number) {
    super(message);
    this.name = "AuthError";
    this.code = code;
    this.retryAfter = retryAfter;
  }
}

/** What the app must remember between opening the provider and coming back from it. */
export type PendingOAuth = { state: string; codeVerifier: string };

export type OAuthStart = { authorizationUrl: string; pending: PendingOAuth };

export type OAuthResult =
  | { type: "signed_in"; signedIn: SignedIn }
  | { type: "linked"; provider: SocialProvider };

/** What the desktop app asks the web app for when it says "continue in the browser". */
export type DesktopHandoff = { redirectUri: string; codeChallenge: string; state?: string };

export type EmailChallenge = { challengeId: string; expiresIn: number; resendAfter: number };

export type AuthClientOptions = {
  /** The gateway's base URL, for example `http://localhost:8787`. */
  baseUrl: string;
  client: ClientKind;
  /**
   * The session token, for the desktop and mobile apps. The web app leaves this out: its
   * session cookie is sent by the browser.
   */
  getToken?: () => string | null | undefined;
  crypto?: CryptoAdapter;
  fetch?: typeof fetch;
};

export type AuthClient = ReturnType<typeof createAuthClient>;

export function createAuthClient(options: AuthClientOptions) {
  const baseUrl = options.baseUrl.replace(/\/+$/, "");
  const crypto = options.crypto ?? webCrypto;
  const doFetch = options.fetch ?? fetch;

  async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
    const headers: Record<string, string> = {};
    if (body !== undefined) headers["content-type"] = "application/json";
    const token = options.getToken?.();
    if (token) headers.authorization = `Bearer ${token}`;

    let response: Response;
    try {
      response = await doFetch(baseUrl + path, {
        method,
        headers,
        body: body === undefined ? undefined : JSON.stringify(body),
        // Only the web app has a cookie to send, and it needs it sent across ports.
        credentials: options.client === "web" ? "include" : "omit",
      });
    } catch (cause) {
      throw new AuthError("network", `could not reach ${baseUrl}: ${String(cause)}`);
    }
    if (response.status === 204) return undefined as T;

    const payload: unknown = await response.json().catch(() => undefined);
    if (response.ok) return payload as T;
    const error = (payload as { error?: { code?: AuthErrorCode; message?: string } } | undefined)
      ?.error;
    if (!error?.code) {
      throw new AuthError("network", `${method} ${path} answered ${response.status}`);
    }
    const retryAfter = Number(response.headers.get("retry-after")) || undefined;
    throw new AuthError(error.code, error.message ?? error.code, retryAfter);
  }

  return {
    /** The sign-in methods this gateway offers. */
    async providers(): Promise<Provider[]> {
      return (await request<{ providers: Provider[] }>("GET", "/v1/auth/providers")).providers;
    },

    /** The signed-in account, or null when there is no valid session. */
    async account(): Promise<Account | null> {
      try {
        return await request<Account>("GET", "/v1/me");
      } catch (error) {
        if (error instanceof AuthError && error.code === "unauthorized") return null;
        throw error;
      }
    },

    /** Sends a six-digit code to the address. With `link`, adds the address to this account. */
    async startEmail(email: string, { link = false } = {}): Promise<EmailChallenge> {
      const challenge = await request<{
        challenge_id: string;
        expires_in: number;
        resend_after: number;
      }>("POST", "/v1/auth/email/start", { email, intent: link ? "link" : "sign_in" });
      return {
        challengeId: challenge.challenge_id,
        expiresIn: challenge.expires_in,
        resendAfter: challenge.resend_after,
      };
    },

    verifyEmail(challengeId: string, code: string): Promise<SignedIn> {
      return request<SignedIn>("POST", "/v1/auth/email/verify", {
        challenge_id: challengeId,
        code,
        client: options.client,
      });
    },

    /**
     * Begins a provider sign-in. Open `authorizationUrl` in a browser and keep `pending` until
     * the browser comes back to `redirectUri`. With `link`, adds the provider to this account.
     */
    async startOAuth(
      provider: SocialProvider,
      { redirectUri, link = false }: { redirectUri: string; link?: boolean },
    ): Promise<OAuthStart> {
      const pending = { state: randomSecret(crypto), codeVerifier: randomSecret(crypto) };
      const started = await request<{ authorization_url: string }>(
        "POST",
        `/v1/auth/oauth/${provider}/start`,
        {
          client: options.client,
          redirect_uri: redirectUri,
          code_challenge: await codeChallenge(pending.codeVerifier, crypto),
          state: pending.state,
          intent: link ? "link" : "sign_in",
        },
      );
      return { authorizationUrl: started.authorization_url, pending };
    },

    /**
     * Finishes a provider sign-in with the URL the browser came back to. Throws the gateway's
     * error when the sign-in failed, and `flow_expired` when the URL is not the answer to
     * `pending`: that is what a forged or stale redirect looks like.
     */
    async finishOAuth(pending: PendingOAuth, callbackUrl: string): Promise<OAuthResult> {
      const params = new URL(callbackUrl).searchParams;
      if (params.get("state") !== pending.state) {
        throw new AuthError("flow_expired", "the redirect does not belong to this sign-in");
      }
      const error = params.get("error");
      if (error) throw new AuthError(error as AuthErrorCode, `sign-in failed: ${error}`);
      const linked = params.get("linked");
      if (linked) return { type: "linked", provider: linked as SocialProvider };
      const code = params.get("code");
      if (!code) throw new AuthError("flow_expired", "the redirect carries no login code");
      const signedIn = await request<SignedIn>("POST", "/v1/auth/token", {
        code,
        code_verifier: pending.codeVerifier,
      });
      return { type: "signed_in", signedIn };
    },

    /**
     * Signs the desktop app in from this session. The desktop app opens the web app with these
     * three values; after the person confirms, send the browser to the returned URL.
     */
    async handoff(handoff: DesktopHandoff): Promise<string> {
      const answer = await request<{ redirect_url: string }>("POST", "/v1/auth/handoff", {
        redirect_uri: handoff.redirectUri,
        code_challenge: handoff.codeChallenge,
        state: handoff.state,
      });
      return answer.redirect_url;
    },

    /** Removes a sign-in method. The last one cannot be removed. */
    unlink(identityId: string): Promise<void> {
      return request<void>("DELETE", `/v1/me/identities/${identityId}`);
    },

    /** Ends the current session on the gateway. */
    signOut(): Promise<void> {
      return request<void>("POST", "/v1/auth/logout", {});
    },
  };
}

import assert from "node:assert/strict";
import { test } from "node:test";
import { AuthError, createAuthClient } from "./client.ts";
import { base64Url, codeChallenge, webCrypto } from "./pkce.ts";

type Call = { url: string; init: RequestInit };

/** A client whose requests are answered by `respond` and recorded in `calls`. */
function stubbed(
  respond: (call: Call) => Response,
  options: { client?: "web" | "mobile"; token?: string } = {},
) {
  const calls: Call[] = [];
  const client = createAuthClient({
    baseUrl: "http://gateway.test/",
    client: options.client ?? "mobile",
    getToken: () => options.token,
    fetch: async (url, init) => {
      const call = { url: String(url), init: init ?? {} };
      calls.push(call);
      return respond(call);
    },
  });
  return { client, calls };
}

const json = (body: unknown, status = 200, headers: Record<string, string> = {}) =>
  new Response(JSON.stringify(body), { status, headers });

const ACCOUNT = { user: { id: "u1", display_name: null, created_at: "" }, identities: [] };

test("base64url matches the standard encoding, unpadded", () => {
  for (const length of [0, 1, 2, 3, 4, 31, 32, 33]) {
    const bytes = Uint8Array.from({ length }, (_, i) => (i * 37 + 251) % 256);
    assert.equal(base64Url(bytes), Buffer.from(bytes).toString("base64url"));
  }
});

test("the code challenge matches the RFC 7636 example", async () => {
  const challenge = await codeChallenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk", webCrypto);
  assert.equal(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
});

test("startOAuth sends a challenge and keeps the verifier", async () => {
  const { client, calls } = stubbed(() => json({ authorization_url: "https://idp.test/a" }));
  const { authorizationUrl, pending } = await client.startOAuth("kakao", {
    redirectUri: "malmoon://auth/callback",
  });
  assert.equal(authorizationUrl, "https://idp.test/a");
  assert.equal(calls[0]?.url, "http://gateway.test/v1/auth/oauth/kakao/start");
  const body = JSON.parse(String(calls[0]?.init.body));
  assert.equal(body.code_challenge, await codeChallenge(pending.codeVerifier, webCrypto));
  assert.equal(body.state, pending.state);
  assert.equal(body.intent, "sign_in");
  assert.equal(JSON.stringify(body).includes(pending.codeVerifier), false);
});

test("finishOAuth trades the code with the verifier", async () => {
  const signedIn = { ...ACCOUNT, session: { token: "mms_t", expires_at: "" } };
  const { client, calls } = stubbed(() => json(signedIn));
  const pending = { state: "s1", codeVerifier: "v1" };
  const result = await client.finishOAuth(pending, "malmoon://auth/callback?code=c1&state=s1");
  assert.deepEqual(result, { type: "signed_in", signedIn });
  assert.deepEqual(JSON.parse(String(calls[0]?.init.body)), { code: "c1", code_verifier: "v1" });
});

test("finishOAuth refuses a redirect that is not the answer to this sign-in", async () => {
  const { client, calls } = stubbed(() => json({}));
  const pending = { state: "s1", codeVerifier: "v1" };
  for (const url of ["malmoon://auth/callback?code=c1&state=other", "malmoon://auth/callback"]) {
    await assert.rejects(client.finishOAuth(pending, url), { code: "flow_expired" });
  }
  assert.equal(calls.length, 0);
});

test("finishOAuth reports the gateway's error and a completed link", async () => {
  const { client } = stubbed(() => json({}));
  const pending = { state: "s1", codeVerifier: "v1" };
  await assert.rejects(
    client.finishOAuth(pending, "http://localhost:5173/auth/callback?error=email_in_use&state=s1"),
    { code: "email_in_use" },
  );
  assert.deepEqual(
    await client.finishOAuth(pending, "http://localhost:5173/auth/callback?linked=line&state=s1"),
    { type: "linked", provider: "line" },
  );
});

test("account is null without a session and throws on other failures", async () => {
  const unauthorized = stubbed(() =>
    json({ error: { code: "unauthorized", message: "sign in" } }, 401),
  );
  assert.equal(await unauthorized.client.account(), null);

  const broken = stubbed(() => new Response("bad gateway", { status: 502 }));
  await assert.rejects(broken.client.account(), { code: "network" });

  const down = createAuthClient({
    baseUrl: "http://gateway.test",
    client: "mobile",
    fetch: async () => {
      throw new TypeError("fetch failed");
    },
  });
  await assert.rejects(down.account(), (error) => {
    return error instanceof AuthError && error.code === "network";
  });
});

test("errors carry the code and the wait", async () => {
  const { client } = stubbed(() =>
    json({ error: { code: "rate_limited", message: "wait" } }, 429, { "retry-after": "42" }),
  );
  await assert.rejects(client.startEmail("mina@example.com"), {
    code: "rate_limited",
    retryAfter: 42,
  });
});

test("native clients send the token, the web client sends the cookie", async () => {
  const sent = async (options: { client?: "web" | "mobile"; token?: string }) => {
    const { client, calls } = stubbed(() => json(ACCOUNT), options);
    await client.account();
    const init = calls[0]?.init ?? {};
    return { headers: init.headers as Record<string, string>, credentials: init.credentials };
  };

  const native = await sent({ token: "mms_t" });
  assert.equal(native.headers.authorization, "Bearer mms_t");
  assert.equal(native.credentials, "omit");

  const web = await sent({ client: "web" });
  assert.equal(web.headers.authorization, undefined);
  assert.equal(web.credentials, "include");
});

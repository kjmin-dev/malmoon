import { type Account, createAuthClient, type SignedIn, type SocialProvider } from "@malmoon/auth";
import * as Crypto from "expo-crypto";
import * as SecureStore from "expo-secure-store";
import * as WebBrowser from "expo-web-browser";
import { Platform } from "react-native";

// The Android emulator reaches the host machine at 10.0.2.2, the iOS simulator at localhost.
const HOST = Platform.OS === "android" ? "10.0.2.2" : "localhost";
export const GATEWAY_URL = process.env.EXPO_PUBLIC_GATEWAY_URL ?? `http://${HOST}:8787`;
export const SESSION_URL = GATEWAY_URL.replace(/^http/, "ws");

/** Registered with the gateway. The scheme is the one in app.json. */
const REDIRECT_URI = "malmoon://auth/callback";
const TOKEN_KEY = "malmoon.session";

/** The session token, also kept in the device's keychain or keystore between launches. */
let token: string | null = null;

export const auth = createAuthClient({
  baseUrl: GATEWAY_URL,
  client: "mobile",
  getToken: () => token,
  crypto: {
    randomBytes: (length) => Crypto.getRandomBytes(length),
    sha256: async (text) =>
      new Uint8Array(
        await Crypto.digest(Crypto.CryptoDigestAlgorithm.SHA256, new TextEncoder().encode(text)),
      ),
  },
});

export type Session = { token: string; account: Account };

/** The session from the last launch, if the gateway still accepts it. */
export async function restoreSession(): Promise<Session | null> {
  token = await SecureStore.getItemAsync(TOKEN_KEY);
  if (!token) return null;
  const account = await auth.account();
  if (!account) {
    await forget();
    return null;
  }
  return { token, account };
}

export async function remember(signedIn: SignedIn): Promise<Session> {
  const issued = signedIn.session?.token;
  if (!issued) throw new Error("the gateway sent no session token to a mobile client");
  token = issued;
  await SecureStore.setItemAsync(TOKEN_KEY, issued);
  return { token: issued, account: { user: signedIn.user, identities: signedIn.identities } };
}

async function forget(): Promise<void> {
  token = null;
  await SecureStore.deleteItemAsync(TOKEN_KEY);
}

export async function signOut(): Promise<void> {
  try {
    await auth.signOut();
  } finally {
    // Signed out on this device even when the gateway cannot be reached.
    await forget();
  }
}

/**
 * Signs in with a provider in the system's in-app browser. Resolves to null when the person
 * closes the browser without finishing.
 */
export async function signInWith(provider: SocialProvider): Promise<Session | null> {
  const { authorizationUrl, pending } = await auth.startOAuth(provider, {
    redirectUri: REDIRECT_URI,
  });
  const result = await WebBrowser.openAuthSessionAsync(authorizationUrl, REDIRECT_URI);
  if (result.type !== "success") return null;
  const finished = await auth.finishOAuth(pending, result.url);
  return finished.type === "signed_in" ? remember(finished.signedIn) : null;
}

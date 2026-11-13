// PKCE (RFC 7636) for the sign-in flow: the app keeps a random verifier and sends only its
// SHA-256, so the login code on the redirect is useless to anyone who did not start the flow.

/** The two primitives PKCE needs. Browsers and Node have them; React Native passes its own. */
export type CryptoAdapter = {
  randomBytes: (length: number) => Uint8Array;
  sha256: (text: string) => Promise<Uint8Array>;
};

export const webCrypto: CryptoAdapter = {
  randomBytes: (length) => crypto.getRandomValues(new Uint8Array(length)),
  sha256: async (text) =>
    new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text))),
};

const ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/** Unpadded base64url. Written out because React Native has neither `btoa` nor `Buffer`. */
export function base64Url(bytes: Uint8Array): string {
  let out = "";
  for (let i = 0; i < bytes.length; i += 3) {
    const a = bytes[i] ?? 0;
    const b = bytes[i + 1];
    const c = bytes[i + 2];
    out += ALPHABET.charAt(a >> 2);
    out += ALPHABET.charAt(((a & 3) << 4) | ((b ?? 0) >> 4));
    if (b !== undefined) out += ALPHABET.charAt(((b & 15) << 2) | ((c ?? 0) >> 6));
    if (c !== undefined) out += ALPHABET.charAt(c & 63);
  }
  return out;
}

export function randomSecret(crypto: CryptoAdapter): string {
  return base64Url(crypto.randomBytes(32));
}

export async function codeChallenge(verifier: string, crypto: CryptoAdapter): Promise<string> {
  return base64Url(await crypto.sha256(verifier));
}

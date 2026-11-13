// The account API for TypeScript clients: sign-in with an email code or a social provider,
// linked sign-in methods, and sign-out. The web and mobile apps share it.

export {
  type Account,
  type AuthClient,
  type AuthClientOptions,
  AuthError,
  type AuthErrorCode,
  type ClientKind,
  createAuthClient,
  type DesktopHandoff,
  type EmailChallenge,
  type Identity,
  type OAuthResult,
  type OAuthStart,
  type PendingOAuth,
  type Provider,
  type SignedIn,
  type SocialProvider,
  type User,
} from "./client.ts";
export { errorMessage, PROVIDER_NAME } from "./messages.ts";
export { base64Url, type CryptoAdapter, codeChallenge, webCrypto } from "./pkce.ts";

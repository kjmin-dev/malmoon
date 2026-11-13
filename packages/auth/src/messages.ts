// The words the apps show for providers and for failed sign-ins. Shared so that the web and
// mobile apps say the same thing.

import { AuthError, type AuthErrorCode, type Provider } from "./client.ts";

export const PROVIDER_NAME: Record<Provider, string> = {
  email: "이메일",
  google: "Google",
  apple: "Apple",
  kakao: "카카오",
  line: "LINE",
};

const ERROR_MESSAGE: Partial<Record<AuthErrorCode, string>> = {
  invalid_request: "입력한 내용을 다시 확인해 주세요.",
  invalid_code: "코드가 맞지 않거나 만료되었습니다.",
  too_many_attempts: "코드를 너무 여러 번 틀렸습니다. 새 코드를 요청해 주세요.",
  rate_limited: "코드를 방금 보냈습니다. 잠시 후 다시 요청해 주세요.",
  email_in_use:
    "이 이메일로 가입한 계정이 이미 있습니다. 기존 방법으로 로그인한 뒤, 계정에서 이 로그인 방법을 연결해 주세요.",
  identity_in_use: "이 로그인 방법은 다른 계정에 연결되어 있습니다.",
  last_identity: "마지막 로그인 방법은 해제할 수 없습니다.",
  cancelled: "로그인을 취소했습니다.",
  flow_expired: "로그인 요청이 만료되었습니다. 다시 시도해 주세요.",
  provider_unavailable: "지금은 이 로그인 방법을 쓸 수 없습니다.",
  provider_error: "로그인 제공자가 요청을 완료하지 못했습니다. 다시 시도해 주세요.",
  network: "서버에 연결할 수 없습니다.",
};

/** What to tell the person when an account request fails. */
export function errorMessage(error: unknown): string {
  const code = error instanceof AuthError ? error.code : undefined;
  return (code && ERROR_MESSAGE[code]) ?? "문제가 생겼습니다. 잠시 후 다시 시도해 주세요.";
}

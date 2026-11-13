import { Redirect } from "expo-router";

/**
 * Where providers send the browser back to (`malmoon://auth/callback`). The sign-in code reads
 * the URL from the browser session; this route only exists so that Android, which also delivers
 * the link to the router, has somewhere to land.
 */
export default function AuthCallback() {
  return <Redirect href="/" />;
}

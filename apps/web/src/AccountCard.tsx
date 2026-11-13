import {
  type Account,
  errorMessage,
  PROVIDER_NAME,
  type Provider,
  type SocialProvider,
} from "@malmoon/auth";
import { Badge, Banner, Button, Card, Text } from "@malmoon/ui";
import { useEffect, useState } from "react";
import { auth, beginOAuth } from "./auth.ts";

/** The signed-in account: its sign-in methods, linking more, and signing out. */
export function AccountCard({
  account,
  onChange,
}: {
  account: Account;
  /** Called with the updated account, or null after signing out. */
  onChange: (account: Account | null) => void;
}) {
  const [providers, setProviders] = useState<Provider[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();

  useEffect(() => {
    auth.providers().then(setProviders, () => {});
  }, []);

  const run = async (action: () => Promise<void>) => {
    setBusy(true);
    setError(undefined);
    try {
      await action();
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setBusy(false);
    }
  };

  const linked = new Set(account.identities.map((identity) => identity.provider));
  const linkable = providers.filter(
    (provider): provider is SocialProvider => provider !== "email" && !linked.has(provider),
  );
  const onlyOne = account.identities.length === 1;

  return (
    <Card
      title="계정"
      actions={
        <Button
          size="sm"
          disabled={busy}
          onPress={() =>
            run(async () => {
              await auth.signOut();
              onChange(null);
            })
          }
        >
          로그아웃
        </Button>
      }
    >
      <ul className="flex flex-col gap-2">
        {account.identities.map((identity) => (
          <li key={identity.id} className="flex items-center justify-between gap-3">
            <span className="flex min-w-0 items-center gap-2">
              <Badge>{PROVIDER_NAME[identity.provider]}</Badge>
              <Text variant="detail" tone="secondary" as="span" className="truncate">
                {identity.email ?? identity.display_name ?? ""}
              </Text>
            </span>
            <Button
              variant="ghost"
              size="sm"
              // An account always keeps one way in.
              disabled={busy || onlyOne}
              onPress={() =>
                run(async () => {
                  await auth.unlink(identity.id);
                  onChange(await auth.account());
                })
              }
            >
              연결 해제
            </Button>
          </li>
        ))}
      </ul>
      {linkable.length > 0 && (
        <div className="flex flex-wrap gap-2">
          {linkable.map((provider) => (
            <Button
              key={provider}
              size="sm"
              disabled={busy}
              onPress={() => run(() => beginOAuth(provider, { link: true }))}
            >
              {PROVIDER_NAME[provider]} 연결
            </Button>
          ))}
        </div>
      )}
      {error && <Banner tone="danger">{error}</Banner>}
    </Card>
  );
}

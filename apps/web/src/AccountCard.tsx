import { Button, Card, Chip } from "@heroui/react";
import type { Account, Provider, SocialProvider } from "@malmoon/auth";
import { errorMessage, PROVIDER_NAME } from "@malmoon/auth";
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
    <Card>
      <Card.Header className="flex-row items-center justify-between">
        <Card.Title>계정</Card.Title>
        <Button
          variant="secondary"
          size="sm"
          isDisabled={busy}
          onPress={() =>
            run(async () => {
              await auth.signOut();
              onChange(null);
            })
          }
        >
          로그아웃
        </Button>
      </Card.Header>
      <Card.Content className="flex flex-col gap-3">
        <ul className="flex flex-col gap-2">
          {account.identities.map((identity) => (
            <li key={identity.id} className="flex items-center justify-between gap-3">
              <span className="flex min-w-0 items-center gap-2">
                <Chip>{PROVIDER_NAME[identity.provider]}</Chip>
                <span className="truncate text-mm-sm text-muted">
                  {identity.email ?? identity.display_name ?? ""}
                </span>
              </span>
              <Button
                variant="ghost"
                size="sm"
                // An account always keeps one way in.
                isDisabled={busy || onlyOne}
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
                variant="secondary"
                size="sm"
                isDisabled={busy}
                onPress={() => run(() => beginOAuth(provider, { link: true }))}
              >
                {PROVIDER_NAME[provider]} 연결
              </Button>
            ))}
          </div>
        )}
        {error && (
          <p role="alert" className="text-mm-sm text-danger">
            {error}
          </p>
        )}
      </Card.Content>
    </Card>
  );
}

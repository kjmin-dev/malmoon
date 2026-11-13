import { Button, Card } from "@heroui/react";
import type { Account, DesktopHandoff } from "@malmoon/auth";
import { errorMessage } from "@malmoon/auth";
import { useState } from "react";
import { auth, forgetDesktopHandoff } from "./auth.ts";

/**
 * Asks before signing the desktop app in with this account. Any program on the computer can open
 * this page, so the session is never handed over without the person saying yes.
 */
export function DesktopSignIn({
  account,
  handoff,
  onDone,
}: {
  account: Account;
  handoff: DesktopHandoff;
  onDone: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();
  const who =
    account.identities.find((identity) => identity.email)?.email ??
    account.user.display_name ??
    "이 계정";

  const approve = async () => {
    setBusy(true);
    setError(undefined);
    try {
      const redirectUrl = await auth.handoff(handoff);
      forgetDesktopHandoff();
      // The desktop app is listening on this loopback address for the login code.
      location.assign(redirectUrl);
    } catch (cause) {
      setError(errorMessage(cause));
      setBusy(false);
    }
  };

  const decline = () => {
    forgetDesktopHandoff();
    onDone();
  };

  return (
    <Card className="mx-auto w-full max-w-sm">
      <Card.Header>
        <Card.Title>데스크톱 앱에 로그인할까요?</Card.Title>
        <Card.Description>
          이 컴퓨터의 Malmoon 데스크톱 앱이 {who}(으)로 로그인합니다. 직접 요청한 것이 아니면
          취소하세요.
        </Card.Description>
      </Card.Header>
      <Card.Content className="flex flex-col gap-2">
        <Button isDisabled={busy} onPress={approve} fullWidth>
          데스크톱 앱에 로그인
        </Button>
        <Button variant="ghost" isDisabled={busy} onPress={decline} fullWidth>
          취소
        </Button>
        {error && (
          <p role="alert" className="text-mm-sm text-danger">
            {error}
          </p>
        )}
      </Card.Content>
    </Card>
  );
}

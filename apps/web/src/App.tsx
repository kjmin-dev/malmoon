import { Button, Card, Chip } from "@heroui/react";
import type { Account } from "@malmoon/auth";
import { useSession } from "@malmoon/session/react";
import { useEffect, useState } from "react";
import { AccountCard } from "./AccountCard.tsx";
import { type Boot, booted, SESSION_URL } from "./auth.ts";
import { Captions } from "./Captions.tsx";
import { DesktopSignIn } from "./DesktopSignIn.tsx";
import { SignIn } from "./SignIn.tsx";

type Theme = "light" | "dark";

export function App() {
  // Undefined until the first answer about who is signed in.
  const [boot, setBoot] = useState<Boot>();
  const [theme, setTheme] = useState(document.documentElement.dataset.theme as Theme);

  useEffect(() => {
    booted.then(setBoot);
  }, []);

  const toggleTheme = () => {
    const next = theme === "dark" ? "light" : "dark";
    document.documentElement.dataset.theme = next;
    setTheme(next);
  };
  const setAccount = (account: Account | null) => setBoot({ account, handoff: boot?.handoff });

  return (
    <main className="mx-auto flex min-h-dvh max-w-3xl flex-col gap-6 p-6">
      <header className="flex items-center justify-between">
        <h1 className="font-mm-bold text-mm-xl">Malmoon</h1>
        <Button variant="secondary" size="sm" onPress={toggleTheme}>
          {theme === "dark" ? "밝은 화면" : "어두운 화면"}
        </Button>
      </header>

      {boot?.notice && (
        <p
          role={boot.notice.tone === "danger" ? "alert" : "status"}
          className={`text-mm-sm ${boot.notice.tone === "danger" ? "text-danger" : "text-success"}`}
        >
          {boot.notice.text}
        </p>
      )}

      {boot === undefined ? null : boot.account === null ? (
        <>
          {boot.handoff && (
            <p className="text-center text-mm-sm text-muted">
              데스크톱 앱에 로그인하려면 먼저 여기에서 로그인해 주세요.
            </p>
          )}
          <SignIn onSignedIn={setAccount} />
        </>
      ) : boot.handoff ? (
        <DesktopSignIn
          account={boot.account}
          handoff={boot.handoff}
          onDone={() => setBoot({ account: boot.account })}
        />
      ) : (
        <>
          <LiveCaptions />
          <AccountCard account={boot.account} onChange={setAccount} />
        </>
      )}
    </main>
  );
}

/** Mounted only while signed in: the gateway refuses sessions without an account. */
function LiveCaptions() {
  const { status, segments } = useSession(SESSION_URL, "web");
  return (
    <Card>
      <Card.Header className="flex-row items-center justify-between">
        <Card.Title>실시간 자막</Card.Title>
        {status.state === "live" && <Chip color="success">연결됨</Chip>}
        {status.state === "connecting" && <Chip color="warning">연결하는 중</Chip>}
        {status.state === "offline" && <Chip color="danger">연결 끊김</Chip>}
      </Card.Header>
      <Card.Content>
        <Captions segments={segments} />
        {status.state === "offline" && (
          <p className="mt-3 text-mm-sm text-muted">
            게이트웨이에 연결할 수 없습니다. <code>{"mise run //apps/gateway:dev"}</code>로
            실행하세요.
          </p>
        )}
      </Card.Content>
    </Card>
  );
}

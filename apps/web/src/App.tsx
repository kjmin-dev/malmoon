import type { Account } from "@malmoon/auth";
import { useSession } from "@malmoon/session/react";
import { Badge, Banner, Button, Card, Text } from "@malmoon/ui";
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
        <Text variant="title" as="h1">
          Malmoon
        </Text>
        <Button variant="ghost" size="sm" onPress={toggleTheme}>
          {theme === "dark" ? "밝은 화면" : "어두운 화면"}
        </Button>
      </header>

      {boot?.notice && (
        <Banner tone={boot.notice.tone === "danger" ? "danger" : "success"}>
          {boot.notice.text}
        </Banner>
      )}

      {boot === undefined ? null : boot.account === null ? (
        <>
          {boot.handoff && (
            <Banner tone="info">데스크톱 앱에 로그인하려면 먼저 여기에서 로그인해 주세요.</Banner>
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
  const badge = {
    live: (
      <Badge tone="success" dot>
        연결됨
      </Badge>
    ),
    connecting: (
      <Badge tone="warning" dot>
        연결하는 중
      </Badge>
    ),
    offline: (
      <Badge tone="danger" dot>
        연결 끊김
      </Badge>
    ),
  }[status.state];
  return (
    <Card title="실시간 자막" actions={badge}>
      <Captions segments={segments} />
      {status.state === "offline" && (
        <Text variant="detail" tone="secondary">
          게이트웨이에 연결할 수 없습니다. <code>{"mise run //apps/gateway:dev"}</code>로
          실행하세요.
        </Text>
      )}
    </Card>
  );
}

import { Button, Card, Chip } from "@heroui/react";
import { useSession } from "@malmoon/session/react";
import { useState } from "react";
import { Captions } from "./Captions.tsx";

const GATEWAY_URL = import.meta.env.VITE_GATEWAY_URL ?? "ws://127.0.0.1:8787";

type Theme = "light" | "dark";

export function App() {
  const { status, segments } = useSession(GATEWAY_URL, "web");
  const [theme, setTheme] = useState(document.documentElement.dataset.theme as Theme);

  const toggleTheme = () => {
    const next = theme === "dark" ? "light" : "dark";
    document.documentElement.dataset.theme = next;
    setTheme(next);
  };

  return (
    <main className="mx-auto flex min-h-dvh max-w-3xl flex-col gap-6 p-6">
      <header className="flex items-center justify-between">
        <h1 className="font-mm-bold text-mm-xl">Malmoon</h1>
        <Button variant="secondary" size="sm" onPress={toggleTheme}>
          {theme === "dark" ? "밝은 화면" : "어두운 화면"}
        </Button>
      </header>

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
    </main>
  );
}

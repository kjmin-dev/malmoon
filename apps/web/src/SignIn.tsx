import { Button, Card, FieldError, Input, Label, Separator, TextField } from "@heroui/react";
import type { Account, EmailChallenge, Provider, SocialProvider } from "@malmoon/auth";
import { errorMessage, PROVIDER_NAME } from "@malmoon/auth";
import { type FormEvent, useEffect, useState } from "react";
import { auth, beginOAuth } from "./auth.ts";

type Step = { name: "email" } | { name: "code"; challenge: EmailChallenge };

export function SignIn({ onSignedIn }: { onSignedIn: (account: Account) => void }) {
  const [providers, setProviders] = useState<Provider[]>(["email"]);
  const [step, setStep] = useState<Step>({ name: "email" });
  const [email, setEmail] = useState("");
  const [code, setCode] = useState("");
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

  // An invalid field blocks its form from being submitted, so the error has to go as soon as
  // the person starts correcting the value.
  const edit = (change: () => void) => {
    setError(undefined);
    change();
  };

  const sendCode = (event: FormEvent) => {
    event.preventDefault();
    run(async () => {
      setStep({ name: "code", challenge: await auth.startEmail(email) });
      setCode("");
    });
  };

  const verify = (event: FormEvent) => {
    event.preventDefault();
    if (step.name !== "code") return;
    run(async () => onSignedIn(await auth.verifyEmail(step.challenge.challengeId, code)));
  };

  const social = providers.filter((provider): provider is SocialProvider => provider !== "email");

  return (
    <Card className="mx-auto w-full max-w-sm">
      <Card.Header>
        <Card.Title>로그인</Card.Title>
        <Card.Description>처음이면 계정이 만들어집니다.</Card.Description>
      </Card.Header>
      <Card.Content className="flex flex-col gap-4">
        {step.name === "email" ? (
          <form className="flex flex-col gap-3" onSubmit={sendCode}>
            <TextField
              name="email"
              type="email"
              autoComplete="email"
              isRequired
              isInvalid={error !== undefined}
              value={email}
              onChange={(value) => edit(() => setEmail(value))}
            >
              <Label>이메일</Label>
              <Input placeholder="name@example.com" />
              <FieldError>{error}</FieldError>
            </TextField>
            <Button type="submit" isDisabled={busy} fullWidth>
              로그인 코드 받기
            </Button>
          </form>
        ) : (
          <form className="flex flex-col gap-3" onSubmit={verify}>
            <TextField
              name="code"
              inputMode="numeric"
              autoComplete="one-time-code"
              autoFocus
              isRequired
              isInvalid={error !== undefined}
              value={code}
              onChange={(value) => edit(() => setCode(value.replace(/\D/g, "").slice(0, 6)))}
            >
              <Label>{email}로 보낸 6자리 코드</Label>
              <Input placeholder="000000" />
              <FieldError>{error}</FieldError>
            </TextField>
            <Button type="submit" isDisabled={busy || code.length !== 6} fullWidth>
              로그인
            </Button>
            <Button
              variant="ghost"
              isDisabled={busy}
              onPress={() => {
                setStep({ name: "email" });
                setError(undefined);
              }}
              fullWidth
            >
              다른 이메일 쓰기
            </Button>
          </form>
        )}

        {social.length > 0 && step.name === "email" && (
          <>
            <Separator />
            <div className="flex flex-col gap-2">
              {social.map((provider) => (
                <Button
                  key={provider}
                  variant="secondary"
                  isDisabled={busy}
                  onPress={() => run(() => beginOAuth(provider))}
                  fullWidth
                >
                  {PROVIDER_NAME[provider]}로 계속하기
                </Button>
              ))}
            </div>
          </>
        )}
      </Card.Content>
    </Card>
  );
}

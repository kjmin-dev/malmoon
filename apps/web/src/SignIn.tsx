import {
  type Account,
  type EmailChallenge,
  errorMessage,
  PROVIDER_NAME,
  type Provider,
  type SocialProvider,
} from "@malmoon/auth";
import { Button, Card, Divider, TextField } from "@malmoon/ui";
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
    <Card
      title="로그인"
      description="처음이면 계정이 만들어집니다."
      className="mx-auto w-full max-w-sm"
    >
      {step.name === "email" ? (
        <form className="flex flex-col gap-3" onSubmit={sendCode}>
          <TextField
            label="이메일"
            name="email"
            type="email"
            autoComplete="email"
            placeholder="name@example.com"
            required
            value={email}
            onChange={(value) => edit(() => setEmail(value))}
            error={error}
          />
          <Button variant="primary" type="submit" loading={busy} fullWidth>
            로그인 코드 받기
          </Button>
        </form>
      ) : (
        <form className="flex flex-col gap-3" onSubmit={verify}>
          <TextField
            label="로그인 코드"
            description={`${email}로 보낸 6자리 숫자입니다.`}
            name="code"
            inputMode="numeric"
            autoComplete="one-time-code"
            placeholder="000000"
            autoFocus
            required
            value={code}
            onChange={(value) => edit(() => setCode(value.replace(/\D/g, "").slice(0, 6)))}
            error={error}
          />
          <Button
            variant="primary"
            type="submit"
            loading={busy}
            disabled={code.length !== 6}
            fullWidth
          >
            로그인
          </Button>
          <Button
            variant="ghost"
            disabled={busy}
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
          <Divider />
          <div className="flex flex-col gap-2">
            {social.map((provider) => (
              <Button
                key={provider}
                disabled={busy}
                onPress={() => run(() => beginOAuth(provider))}
                fullWidth
              >
                {PROVIDER_NAME[provider]}로 계속하기
              </Button>
            ))}
          </div>
        </>
      )}
    </Card>
  );
}

import {
  type EmailChallenge,
  errorMessage,
  PROVIDER_NAME,
  type Provider,
  type SocialProvider,
} from "@malmoon/auth";
import { space } from "@malmoon/tokens";
import { Banner, Button, Divider, Text, TextField } from "@malmoon/ui-native";
import { useEffect, useState } from "react";
import { StyleSheet, View } from "react-native";
import { auth, remember, type Session, signInWith } from "./auth.ts";

type Step = { name: "email" } | { name: "code"; challenge: EmailChallenge };

export function SignInScreen({ onSignedIn }: { onSignedIn: (session: Session) => void }) {
  const [providers, setProviders] = useState<Provider[]>(["email"]);
  const [step, setStep] = useState<Step>({ name: "email" });
  const [email, setEmail] = useState("");
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState(false);
  /** A problem with what was typed, shown under the field. */
  const [fieldError, setFieldError] = useState<string>();
  /** A problem with a provider sign-in, which has no field to sit under. */
  const [providerError, setProviderError] = useState<string>();

  useEffect(() => {
    auth.providers().then(setProviders, () => {});
  }, []);

  const run = async (action: () => Promise<void>, report: (message: string) => void) => {
    setBusy(true);
    setFieldError(undefined);
    setProviderError(undefined);
    try {
      await action();
    } catch (cause) {
      report(errorMessage(cause));
    } finally {
      setBusy(false);
    }
  };

  const sendCode = () =>
    run(async () => {
      setStep({ name: "code", challenge: await auth.startEmail(email) });
      setCode("");
    }, setFieldError);

  const verify = () => {
    if (step.name !== "code") return;
    run(
      async () =>
        onSignedIn(await remember(await auth.verifyEmail(step.challenge.challengeId, code))),
      setFieldError,
    );
  };

  const withProvider = (provider: SocialProvider) =>
    run(async () => {
      const session = await signInWith(provider);
      if (session) onSignedIn(session);
    }, setProviderError);

  const social = providers.filter((provider): provider is SocialProvider => provider !== "email");

  return (
    <View style={styles.form}>
      <View>
        <Text variant="title" heading>
          로그인
        </Text>
        <Text variant="detail" tone="secondary">
          처음이면 계정이 만들어집니다.
        </Text>
      </View>

      {step.name === "email" ? (
        <>
          <TextField
            label="이메일"
            placeholder="name@example.com"
            value={email}
            onChange={(value) => {
              setFieldError(undefined);
              setEmail(value);
            }}
            onSubmit={sendCode}
            error={fieldError}
            autoCapitalize="none"
            autoComplete="email"
            keyboardType="email-address"
            textContentType="emailAddress"
            returnKeyType="send"
          />
          <Button
            variant="primary"
            label="로그인 코드 받기"
            onPress={sendCode}
            loading={busy}
            disabled={email === ""}
            fullWidth
          />
        </>
      ) : (
        <>
          <TextField
            label="로그인 코드"
            description={`${email}로 보낸 6자리 숫자입니다.`}
            placeholder="000000"
            value={code}
            onChange={(value) => {
              setFieldError(undefined);
              setCode(value.replace(/\D/g, "").slice(0, 6));
            }}
            onSubmit={verify}
            error={fieldError}
            autoFocus
            autoComplete="one-time-code"
            keyboardType="number-pad"
            textContentType="oneTimeCode"
          />
          <Button
            variant="primary"
            label="로그인"
            onPress={verify}
            loading={busy}
            disabled={code.length !== 6}
            fullWidth
          />
          <Button
            variant="ghost"
            label="다른 이메일 쓰기"
            onPress={() => {
              setStep({ name: "email" });
              setFieldError(undefined);
            }}
            disabled={busy}
            fullWidth
          />
        </>
      )}

      {step.name === "email" && social.length > 0 && (
        <>
          <Divider />
          {social.map((provider) => (
            <Button
              key={provider}
              label={`${PROVIDER_NAME[provider]}로 계속하기`}
              onPress={() => withProvider(provider)}
              disabled={busy}
              fullWidth
            />
          ))}
        </>
      )}
      {providerError && <Banner tone="danger">{providerError}</Banner>}
    </View>
  );
}

const styles = StyleSheet.create({
  form: {
    gap: space[3],
  },
});

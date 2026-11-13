import {
  type EmailChallenge,
  errorMessage,
  PROVIDER_NAME,
  type Provider,
  type SocialProvider,
} from "@malmoon/auth";
import { colors, font, radius, space } from "@malmoon/tokens";
import { useEffect, useState } from "react";
import { Pressable, StyleSheet, Text, TextInput, useColorScheme, View } from "react-native";
import { auth, remember, type Session, signInWith } from "./auth.ts";

type Step = { name: "email" } | { name: "code"; challenge: EmailChallenge };

export function SignInScreen({ onSignedIn }: { onSignedIn: (session: Session) => void }) {
  const palette = colors[useColorScheme() === "dark" ? "dark" : "light"];
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

  const sendCode = () =>
    run(async () => {
      setStep({ name: "code", challenge: await auth.startEmail(email) });
      setCode("");
    });

  const verify = () => {
    if (step.name !== "code") return;
    run(async () => {
      onSignedIn(await remember(await auth.verifyEmail(step.challenge.challengeId, code)));
    });
  };

  const withProvider = (provider: SocialProvider) =>
    run(async () => {
      const session = await signInWith(provider);
      if (session) onSignedIn(session);
    });

  const input = [
    styles.input,
    {
      color: palette.textPrimary,
      borderColor: error ? palette.statusDanger : palette.borderDefault,
    },
  ];
  const social = providers.filter((provider): provider is SocialProvider => provider !== "email");

  return (
    <View style={styles.form}>
      <Text style={[styles.heading, { color: palette.textPrimary }]}>로그인</Text>
      <Text style={{ color: palette.textSecondary }}>처음이면 계정이 만들어집니다.</Text>

      {step.name === "email" ? (
        <>
          <TextInput
            accessibilityLabel="이메일"
            style={input}
            value={email}
            onChangeText={setEmail}
            onSubmitEditing={sendCode}
            placeholder="name@example.com"
            placeholderTextColor={palette.textSecondary}
            autoCapitalize="none"
            autoComplete="email"
            keyboardType="email-address"
            textContentType="emailAddress"
          />
          <Button
            label="로그인 코드 받기"
            onPress={sendCode}
            disabled={busy || email === ""}
            primary
          />
        </>
      ) : (
        <>
          <Text style={{ color: palette.textSecondary }}>{email}로 보낸 6자리 코드</Text>
          <TextInput
            accessibilityLabel="로그인 코드"
            style={input}
            value={code}
            onChangeText={(value) => setCode(value.replace(/\D/g, "").slice(0, 6))}
            onSubmitEditing={verify}
            placeholder="000000"
            placeholderTextColor={palette.textSecondary}
            autoFocus
            autoComplete="one-time-code"
            keyboardType="number-pad"
            textContentType="oneTimeCode"
          />
          <Button label="로그인" onPress={verify} disabled={busy || code.length !== 6} primary />
          <Button
            label="다른 이메일 쓰기"
            onPress={() => setStep({ name: "email" })}
            disabled={busy}
          />
        </>
      )}

      {error && (
        <Text accessibilityRole="alert" style={{ color: palette.statusDanger }}>
          {error}
        </Text>
      )}

      {step.name === "email" &&
        social.map((provider) => (
          <Button
            key={provider}
            label={`${PROVIDER_NAME[provider]}로 계속하기`}
            onPress={() => withProvider(provider)}
            disabled={busy}
          />
        ))}
    </View>
  );
}

export function Button({
  label,
  onPress,
  disabled = false,
  primary = false,
}: {
  label: string;
  onPress: () => void;
  disabled?: boolean;
  primary?: boolean;
}) {
  const palette = colors[useColorScheme() === "dark" ? "dark" : "light"];
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityState={{ disabled }}
      disabled={disabled}
      onPress={onPress}
      style={({ pressed }) => [
        styles.button,
        primary
          ? { backgroundColor: palette.accentDefault }
          : { borderWidth: StyleSheet.hairlineWidth, borderColor: palette.borderDefault },
        (disabled || pressed) && styles.dimmed,
      ]}
    >
      <Text
        style={[
          styles.buttonLabel,
          { color: primary ? palette.textOnAccent : palette.textPrimary },
        ]}
      >
        {label}
      </Text>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  form: {
    gap: space[3],
  },
  heading: {
    fontSize: font.size.xl,
    fontWeight: font.weight.bold,
  },
  input: {
    // 48 points tall: a comfortable touch target on both platforms.
    minHeight: 48,
    paddingHorizontal: space[3],
    borderWidth: 1,
    borderRadius: radius.md,
    fontSize: font.size.md,
  },
  button: {
    minHeight: 48,
    alignItems: "center",
    justifyContent: "center",
    paddingHorizontal: space[4],
    borderRadius: radius.md,
  },
  buttonLabel: {
    fontSize: font.size.md,
    fontWeight: font.weight.semibold,
  },
  dimmed: {
    opacity: 0.5,
  },
});

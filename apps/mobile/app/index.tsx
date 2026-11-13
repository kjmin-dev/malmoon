import { errorMessage } from "@malmoon/auth";
import { useSession } from "@malmoon/session/react";
import { colors, font, radius, space } from "@malmoon/tokens";
import { useCallback, useEffect, useState } from "react";
import { StyleSheet, Text, useColorScheme, View } from "react-native";
import { SafeAreaView } from "react-native-safe-area-context";
import { restoreSession, SESSION_URL, type Session, signOut } from "../src/auth.ts";
import { Button, SignInScreen } from "../src/SignInScreen.tsx";

const STATUS_LABEL = {
  connecting: "연결하는 중",
  live: "연결됨",
  offline: "연결 끊김",
} as const;

type State =
  | { name: "loading" }
  | { name: "signed_out" }
  | { name: "signed_in"; session: Session }
  /** The gateway could not say whether the saved session is still good. */
  | { name: "unreachable"; message: string };

export default function Home() {
  const palette = colors[useColorScheme() === "dark" ? "dark" : "light"];
  const [state, setState] = useState<State>({ name: "loading" });

  const restore = useCallback(() => {
    setState({ name: "loading" });
    restoreSession().then(
      (session) => setState(session ? { name: "signed_in", session } : { name: "signed_out" }),
      (error) => setState({ name: "unreachable", message: errorMessage(error) }),
    );
  }, []);
  useEffect(restore, [restore]);

  return (
    <SafeAreaView style={[styles.screen, { backgroundColor: palette.backgroundBase }]}>
      {state.name !== "signed_in" && (
        <Text style={[styles.title, { color: palette.textPrimary }]}>Malmoon</Text>
      )}
      {state.name === "signed_out" && (
        <SignInScreen onSignedIn={(session) => setState({ name: "signed_in", session })} />
      )}
      {state.name === "unreachable" && (
        <>
          <Text style={{ color: palette.textSecondary }}>{state.message}</Text>
          <Button label="다시 시도" onPress={restore} />
        </>
      )}
      {state.name === "signed_in" && (
        <Captions
          session={state.session}
          onSignOut={() => signOut().finally(() => setState({ name: "signed_out" }))}
        />
      )}
    </SafeAreaView>
  );
}

/** Mounted only while signed in: the gateway refuses sessions without an account. */
function Captions({ session, onSignOut }: { session: Session; onSignOut: () => void }) {
  const palette = colors[useColorScheme() === "dark" ? "dark" : "light"];
  const { status, segments } = useSession(SESSION_URL, "mobile", session.token);
  const statusColor = {
    connecting: palette.statusWarning,
    live: palette.statusSuccess,
    offline: palette.statusDanger,
  }[status.state];

  return (
    <>
      <View style={styles.header}>
        <Text style={[styles.title, { color: palette.textPrimary }]}>Malmoon</Text>
        <Text style={[styles.status, { color: statusColor }]}>{STATUS_LABEL[status.state]}</Text>
      </View>

      {/* The prompter surface is dark in both color schemes, like the desktop overlay. */}
      <View style={[styles.captions, { backgroundColor: palette.overlaySurface }]}>
        {segments.length === 0 && (
          <Text style={{ color: palette.overlayTextDraft }}>자막을 기다리는 중</Text>
        )}
        {segments.map((segment) => (
          <View key={segment.segment_id}>
            <Text
              // Break Korean between words, not between syllables. Android has no equivalent yet.
              lineBreakStrategyIOS="hangul-word"
              style={[
                styles.original,
                {
                  color:
                    segment.stage === "draft"
                      ? palette.overlayTextDraft
                      : palette.overlayTextPrimary,
                },
              ]}
            >
              {segment.original}
            </Text>
            {segment.translation && (
              <Text
                lineBreakStrategyIOS="hangul-word"
                style={[styles.translation, { color: palette.overlayTextDraft }]}
              >
                {segment.translation}
              </Text>
            )}
          </View>
        ))}
      </View>

      {status.state === "offline" && (
        <Text style={[styles.hint, { color: palette.textSecondary }]}>
          {"게이트웨이에 연결할 수 없습니다. `mise run //apps/gateway:dev`로 실행하세요."}
        </Text>
      )}
      <Button label="로그아웃" onPress={onSignOut} />
    </>
  );
}

const styles = StyleSheet.create({
  screen: {
    flex: 1,
    padding: space[4],
    gap: space[4],
  },
  header: {
    flexDirection: "row",
    alignItems: "center",
    justifyContent: "space-between",
  },
  title: {
    fontSize: font.size.xl,
    fontWeight: font.weight.bold,
  },
  status: {
    fontSize: font.size.sm,
    fontWeight: font.weight.semibold,
  },
  captions: {
    minHeight: 240,
    justifyContent: "flex-end",
    gap: space[3],
    padding: space[4],
    borderRadius: radius.lg,
  },
  original: {
    fontSize: font.size.caption,
    lineHeight: font.size.caption * font.lineHeight.tight,
    fontWeight: font.weight.semibold,
  },
  translation: {
    fontSize: font.size.md,
    lineHeight: font.size.md * font.lineHeight.normal,
  },
  hint: {
    fontSize: font.size.sm,
  },
});

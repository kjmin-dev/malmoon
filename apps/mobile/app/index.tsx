import { errorMessage } from "@malmoon/auth";
import { useSession } from "@malmoon/session/react";
import { colors, radius, space, typography } from "@malmoon/tokens";
import { Badge, Banner, Button, Text, useColors } from "@malmoon/ui-native";
import { useRouter } from "expo-router";
import { useCallback, useEffect, useState } from "react";
import { Text as NativeText, StyleSheet, View } from "react-native";
import { SafeAreaView } from "react-native-safe-area-context";
import { restoreSession, SESSION_URL, type Session, signOut } from "../src/auth.ts";
import { SignInScreen } from "../src/SignInScreen.tsx";

type State =
  | { name: "loading" }
  | { name: "signed_out" }
  | { name: "signed_in"; session: Session }
  /** The gateway could not say whether the saved session is still good. */
  | { name: "unreachable"; message: string };

export default function Home() {
  const palette = useColors();
  const router = useRouter();
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
        <Text variant="display" heading>
          Malmoon
        </Text>
      )}
      {state.name === "signed_out" && (
        <SignInScreen onSignedIn={(session) => setState({ name: "signed_in", session })} />
      )}
      {state.name === "unreachable" && (
        <>
          <Banner tone="danger">{state.message}</Banner>
          <Button label="다시 시도" onPress={restore} />
        </>
      )}
      {state.name === "signed_in" && (
        <Captions
          session={state.session}
          onSignOut={() => signOut().finally(() => setState({ name: "signed_out" }))}
        />
      )}
      {/* In development builds, a way to the design preview. */}
      {__DEV__ && state.name !== "loading" && (
        <Button
          variant="ghost"
          size="sm"
          label="디자인 시스템 보기"
          onPress={() => router.push("/design")}
          style={styles.previewLink}
        />
      )}
    </SafeAreaView>
  );
}

const STATUS = {
  connecting: { tone: "warning", label: "연결하는 중" },
  live: { tone: "success", label: "연결됨" },
  offline: { tone: "danger", label: "연결 끊김" },
} as const;

/** Mounted only while signed in: the gateway refuses sessions without an account. */
function Captions({ session, onSignOut }: { session: Session; onSignOut: () => void }) {
  const { status, segments } = useSession(SESSION_URL, "mobile", session.token);
  // The prompter surface is dark in both color schemes, like the desktop overlay.
  const overlay = colors.dark;

  return (
    <>
      <View style={styles.header}>
        <Text variant="title" heading>
          Malmoon
        </Text>
        <Badge tone={STATUS[status.state].tone} label={STATUS[status.state].label} dot />
      </View>

      <View style={[styles.captions, { backgroundColor: overlay.overlaySurface }]}>
        {segments.length === 0 && (
          <NativeText style={[typography.body, { color: overlay.overlayTextDraft }]}>
            자막을 기다리는 중
          </NativeText>
        )}
        {segments.map((segment) => (
          <View key={segment.segment_id}>
            <NativeText
              // Break Korean between words, not between syllables. Android has no equivalent yet.
              lineBreakStrategyIOS="hangul-word"
              style={[
                typography.caption,
                {
                  color:
                    segment.stage === "draft"
                      ? overlay.overlayTextDraft
                      : overlay.overlayTextPrimary,
                },
              ]}
            >
              {segment.original}
            </NativeText>
            {segment.translation && (
              <NativeText
                lineBreakStrategyIOS="hangul-word"
                style={[typography.body, { color: overlay.overlayTextDraft }]}
              >
                {segment.translation}
              </NativeText>
            )}
          </View>
        ))}
      </View>

      {status.state === "offline" && (
        <Text variant="detail" tone="secondary">
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
  previewLink: {
    marginTop: "auto",
    alignSelf: "center",
  },
  header: {
    flexDirection: "row",
    alignItems: "center",
    justifyContent: "space-between",
  },
  captions: {
    minHeight: 240,
    justifyContent: "flex-end",
    gap: space[3],
    padding: space[4],
    borderRadius: radius.lg,
  },
});

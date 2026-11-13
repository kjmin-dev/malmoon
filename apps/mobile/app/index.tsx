import { useSession } from "@malmoon/session/react";
import { colors, font, radius, space } from "@malmoon/tokens";
import { Platform, StyleSheet, Text, useColorScheme, View } from "react-native";
import { SafeAreaView } from "react-native-safe-area-context";

// The Android emulator reaches the host machine at 10.0.2.2, the iOS simulator at localhost.
const GATEWAY_URL =
  process.env.EXPO_PUBLIC_GATEWAY_URL ??
  (Platform.OS === "android" ? "ws://10.0.2.2:8787" : "ws://127.0.0.1:8787");

const STATUS_LABEL = {
  connecting: "연결하는 중",
  live: "연결됨",
  offline: "연결 끊김",
} as const;

export default function Home() {
  const palette = colors[useColorScheme() === "dark" ? "dark" : "light"];
  const { status, segments } = useSession(GATEWAY_URL, "mobile");
  const statusColor = {
    connecting: palette.statusWarning,
    live: palette.statusSuccess,
    offline: palette.statusDanger,
  }[status.state];

  return (
    <SafeAreaView style={[styles.screen, { backgroundColor: palette.backgroundBase }]}>
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
    </SafeAreaView>
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

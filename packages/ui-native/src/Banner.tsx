import { radius, space, typography } from "@malmoon/tokens";
import type { ReactNode } from "react";
import { Text as NativeText, StyleSheet, View } from "react-native";
import { Text } from "./Text.tsx";
import { type Colors, useColors } from "./theme.ts";

export type BannerTone = "info" | "success" | "warning" | "danger";

/** Each tone has its own mark, so the tone does not depend on color. */
const MARK: Record<BannerTone, string> = { info: "i", success: "✓", warning: "!", danger: "×" };

function look(colors: Colors, tone: BannerTone): { background: string; accent: string } {
  switch (tone) {
    case "info":
      return { background: colors.bannerInfoBackground, accent: colors.bannerInfoAccent };
    case "success":
      return { background: colors.bannerSuccessBackground, accent: colors.bannerSuccessAccent };
    case "warning":
      return { background: colors.bannerWarningBackground, accent: colors.bannerWarningAccent };
    case "danger":
      return { background: colors.bannerDangerBackground, accent: colors.bannerDangerAccent };
  }
}

/**
 * A message about the whole screen or a section of it. Problems (`warning`, `danger`) are
 * announced by the screen reader as soon as they appear.
 */
export function Banner({
  tone = "info",
  title,
  children,
}: {
  tone?: BannerTone;
  title?: string;
  children: ReactNode;
}) {
  const colors = useColors();
  const { background, accent } = look(colors, tone);
  const urgent = tone === "danger" || tone === "warning";
  return (
    <View
      {...(urgent ? { accessibilityRole: "alert" as const } : {})}
      accessibilityLiveRegion={urgent ? "assertive" : "polite"}
      style={[styles.banner, { backgroundColor: background }]}
    >
      <View style={[styles.mark, { backgroundColor: accent }]}>
        <NativeText style={[styles.markText, { color: background }]}>{MARK[tone]}</NativeText>
      </View>
      <View style={styles.content}>
        {title && (
          <Text variant="label" tone="inherit" style={{ color: colors.bannerText }}>
            {title}
          </Text>
        )}
        <Text variant="detail" tone="inherit" style={{ color: colors.bannerText }}>
          {children}
        </Text>
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  banner: {
    alignSelf: "stretch",
    flexDirection: "row",
    alignItems: "flex-start",
    gap: space[3],
    padding: space[3],
    borderRadius: radius.md,
  },
  mark: {
    width: 20,
    height: 20,
    alignItems: "center",
    justifyContent: "center",
    borderRadius: radius.full,
  },
  markText: {
    fontSize: typography.footnote.fontSize,
    fontWeight: typography.footnote.fontWeight,
    lineHeight: 16,
  },
  content: {
    flex: 1,
    gap: 2,
  },
});

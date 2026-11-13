import { radius, space } from "@malmoon/tokens";
import { StyleSheet, View } from "react-native";
import { Text } from "./Text.tsx";
import { type Colors, useColors } from "./theme.ts";

export type BadgeTone = "neutral" | "accent" | "success" | "warning" | "danger";

function look(colors: Colors, tone: BadgeTone): { background: string; text: string } {
  switch (tone) {
    case "neutral":
      return { background: colors.badgeNeutralBackground, text: colors.badgeNeutralText };
    case "accent":
      return { background: colors.badgeAccentBackground, text: colors.badgeAccentText };
    case "success":
      return { background: colors.badgeSuccessBackground, text: colors.badgeSuccessText };
    case "warning":
      return { background: colors.badgeWarningBackground, text: colors.badgeWarningText };
    case "danger":
      return { background: colors.badgeDangerBackground, text: colors.badgeDangerText };
  }
}

/**
 * A short status or category. The words carry the meaning; the color only reinforces it. `dot`
 * adds a small mark for live states such as "connected".
 */
export function Badge({
  tone = "neutral",
  dot = false,
  label,
}: {
  tone?: BadgeTone;
  dot?: boolean;
  label: string;
}) {
  const { background, text } = look(useColors(), tone);
  return (
    <View style={[styles.badge, { backgroundColor: background }]}>
      {dot && <View style={[styles.dot, { backgroundColor: text }]} />}
      <Text variant="footnote" tone="inherit" numberOfLines={1} style={{ color: text }}>
        {label}
      </Text>
    </View>
  );
}

const styles = StyleSheet.create({
  badge: {
    alignSelf: "flex-start",
    flexDirection: "row",
    alignItems: "center",
    gap: 6,
    minHeight: 24,
    paddingHorizontal: space[2],
    borderRadius: radius.sm,
  },
  dot: {
    width: 6,
    height: 6,
    borderRadius: radius.full,
  },
});

import { border, radius, space } from "@malmoon/tokens";
import type { ReactNode } from "react";
import { type StyleProp, StyleSheet, View, type ViewStyle } from "react-native";
import { Text } from "./Text.tsx";
import { useColors } from "./theme.ts";

/**
 * A surface that groups related content. The outline, not a shadow, sets it apart: shadows
 * barely show in dark mode. `title` is a heading; `actions` sit across from it.
 */
export function Card({
  title,
  description,
  actions,
  style,
  children,
}: {
  title?: string;
  description?: string;
  actions?: ReactNode;
  style?: StyleProp<ViewStyle>;
  children?: ReactNode;
}) {
  const colors = useColors();
  return (
    <View
      style={[
        styles.card,
        { backgroundColor: colors.backgroundSurface, borderColor: colors.borderDefault },
        style,
      ]}
    >
      {(title || actions) && (
        <View style={styles.header}>
          <View style={styles.titles}>
            {title && (
              <Text variant="heading" heading>
                {title}
              </Text>
            )}
            {description && (
              <Text variant="detail" tone="secondary">
                {description}
              </Text>
            )}
          </View>
          {actions}
        </View>
      )}
      {children}
    </View>
  );
}

/** A line between groups of content. */
export function Divider() {
  const colors = useColors();
  return <View style={[styles.divider, { backgroundColor: colors.borderDefault }]} />;
}

const styles = StyleSheet.create({
  card: {
    alignSelf: "stretch",
    gap: space[4],
    padding: space[4],
    borderWidth: border.width.thin,
    borderRadius: radius.lg,
  },
  header: {
    flexDirection: "row",
    alignItems: "flex-start",
    justifyContent: "space-between",
    gap: space[3],
  },
  titles: {
    flex: 1,
    gap: 2,
  },
  divider: {
    alignSelf: "stretch",
    height: border.width.thin,
  },
});

import { colors, size } from "@malmoon/tokens";
import { useColorScheme } from "react-native";

/** The color tokens for the system's color scheme. */
export function useColors() {
  return colors[useColorScheme() === "dark" ? "dark" : "light"];
}

export type Colors = ReturnType<typeof useColors>;

/** Phones and tablets are used with fingers, so they take the touch sizes. */
export const control = size.touch;

export type ControlSize = "sm" | "md" | "lg";

export const CONTROL_HEIGHT: Record<ControlSize, number> = {
  sm: control.controlSm,
  md: control.controlMd,
  lg: control.controlLg,
};

import { Button as HeroButton } from "@heroui/react";
import type { ReactNode } from "react";
import { Spinner } from "./Spinner.tsx";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";
export type ControlSize = "sm" | "md" | "lg";

export type ButtonProps = {
  /**
   * `primary` is the one action a view is for; use it once per view. `secondary` is any other
   * action, `ghost` an action that should stay out of the way, `danger` one that destroys.
   */
  variant?: ButtonVariant;
  size?: ControlSize;
  disabled?: boolean;
  /** Shows a spinner and ignores presses. The label stays, so the button keeps its width. */
  loading?: boolean;
  fullWidth?: boolean;
  type?: "button" | "submit" | "reset";
  onPress?: () => void;
  /** Names the button for assistive technology when it has no text, such as an icon button. */
  "aria-label"?: string;
  children: ReactNode;
};

export function Button({
  variant = "secondary",
  size = "md",
  disabled = false,
  loading = false,
  fullWidth = false,
  type = "button",
  onPress,
  children,
  ...aria
}: ButtonProps) {
  return (
    <HeroButton
      {...aria}
      variant={variant}
      size={size}
      type={type}
      isDisabled={disabled}
      isPending={loading}
      fullWidth={fullWidth}
      {...(onPress ? { onPress } : {})}
    >
      {loading && <Spinner size="sm" />}
      {children}
    </HeroButton>
  );
}

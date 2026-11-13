// The design preview: every token and every component in every variant and state. The home
// screen links here; the web app has the same page at `/design`, and the desktop app opens it
// with `malmoon --design`.
//
// `/design?from=3` starts at the third section, for taking screenshots from a script, which
// cannot scroll.

import { radius, size, space, colors as tokenColors, typography } from "@malmoon/tokens";
import {
  Badge,
  type BadgeTone,
  Banner,
  type BannerTone,
  Button,
  type ButtonVariant,
  Card,
  type ControlSize,
  Divider,
  Spinner,
  Text,
  TextField,
  type TextVariant,
  useColors,
} from "@malmoon/ui-native";
import { Stack, useLocalSearchParams } from "expo-router";
import { type ReactNode, useState } from "react";
import { ScrollView, StyleSheet, View } from "react-native";
import { SafeAreaView } from "react-native-safe-area-context";

const VARIANTS: ButtonVariant[] = ["primary", "secondary", "ghost", "danger"];
const SIZES: ControlSize[] = ["sm", "md", "lg"];
const BADGE_TONES: BadgeTone[] = ["neutral", "accent", "success", "warning", "danger"];
const BANNER_TONES: BannerTone[] = ["info", "success", "warning", "danger"];
const TEXT_VARIANTS: TextVariant[] = [
  "display",
  "title",
  "heading",
  "body",
  "body-strong",
  "label",
  "detail",
  "footnote",
  "caption",
];

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <View style={styles.section}>
      <Text variant="title" heading>
        {title}
      </Text>
      {children}
    </View>
  );
}

const kebab = (name: string) => name.replace(/[A-Z0-9]+/g, (match) => `-${match.toLowerCase()}`);

/** Semantic colors by group. Component colors are shown by the components that use them. */
const COMPONENT_COLORS = /^(button|field|badge|banner)/;
const COLOR_NAMES = (Object.keys(tokenColors.light) as (keyof typeof tokenColors.light)[]).filter(
  (name) => !COMPONENT_COLORS.test(name),
);

export default function Design() {
  const colors = useColors();
  const [value, setValue] = useState("");
  const from = Number(useLocalSearchParams<{ from?: string }>().from ?? 1);

  const sections = [
    <Section key="colors" title="색">
      <View style={styles.swatches}>
        {COLOR_NAMES.map((name) => (
          <View key={name} style={styles.swatch}>
            <View
              style={[
                styles.chip,
                { backgroundColor: colors[name], borderColor: colors.borderDefault },
              ]}
            />
            <Text variant="footnote" tone="secondary" style={styles.swatchName}>
              {kebab(name)}
            </Text>
          </View>
        ))}
      </View>
    </Section>,

    <Section key="text" title="글자">
      {TEXT_VARIANTS.map((variant) => (
        <Text key={variant} variant={variant}>
          {variant} · 회의 자막 Malmoon 123
        </Text>
      ))}
    </Section>,

    <Section key="sizes" title="간격, 모서리, 크기">
      <View style={styles.row}>
        {Object.entries(space).map(([name, points]) => (
          <View key={name} style={styles.sample}>
            <View
              style={{ width: points, height: points, backgroundColor: colors.accentDefault }}
            />
            <Text variant="footnote" tone="secondary">
              {name}
            </Text>
          </View>
        ))}
      </View>
      <View style={styles.row}>
        {Object.entries(radius).map(([name, points]) => (
          <View
            key={name}
            style={[styles.radius, { borderRadius: points, borderColor: colors.borderStrong }]}
          >
            <Text variant="footnote" tone="secondary">
              {name}
            </Text>
          </View>
        ))}
      </View>
      <Text variant="detail" tone="secondary">
        본문 {typography.body.fontSize}/{typography.body.lineHeight}. 컨트롤 높이는 터치{" "}
        {size.touch.controlSm}/{size.touch.controlMd}/{size.touch.controlLg}, 마우스{" "}
        {size.pointer.controlSm}/{size.pointer.controlMd}/{size.pointer.controlLg}. 모바일 앱은 터치
        쪽을 쓴다.
      </Text>
    </Section>,

    <Section key="buttons" title="버튼">
      {VARIANTS.map((variant) => (
        <View key={variant} style={styles.row}>
          {SIZES.map((buttonSize) => (
            <Button key={buttonSize} variant={variant} size={buttonSize} label="시작" />
          ))}
          <Button variant={variant} label="불가" disabled />
          <Button variant={variant} label="연결 중" loading />
        </View>
      ))}
      <Button variant="primary" label="전체 너비" fullWidth />
    </Section>,

    <Section key="fields" title="입력">
      <TextField
        label="이메일"
        placeholder="name@example.com"
        description="로그인 코드를 보낼 주소입니다."
        value={value}
        onChange={setValue}
      />
      <TextField
        label="로그인 코드"
        value="12345"
        onChange={() => {}}
        error="코드가 맞지 않거나 만료되었습니다."
      />
      <TextField label="조직" value="Malmoon" onChange={() => {}} disabled />
    </Section>,

    <Section key="badges" title="배지">
      <View style={styles.row}>
        {BADGE_TONES.map((tone) => (
          <Badge key={tone} tone={tone} label={tone} />
        ))}
      </View>
      <View style={styles.row}>
        <Badge tone="success" label="연결됨" dot />
        <Badge tone="warning" label="연결하는 중" dot />
        <Badge tone="danger" label="연결 끊김" dot />
      </View>
    </Section>,

    <Section key="banners" title="배너">
      {BANNER_TONES.map((tone) => (
        <Banner key={tone} tone={tone} title={tone}>
          자막은 이 기기에만 저장됩니다.
        </Banner>
      ))}
    </Section>,

    <Section key="card" title="카드">
      <Card
        title="실시간 자막"
        description="게이트웨이가 보내는 자막을 표시합니다."
        actions={<Badge tone="success" label="연결됨" dot />}
      >
        <Text tone="secondary">카드는 관련된 내용을 하나로 묶는다.</Text>
        <Divider />
        <View style={styles.row}>
          <Spinner size="sm" />
          <Spinner label="불러오는 중" />
        </View>
      </Card>
    </Section>,
  ];

  return (
    <SafeAreaView
      edges={["bottom", "left", "right"]}
      style={{ flex: 1, backgroundColor: colors.backgroundBase }}
    >
      {/* The one screen with a navigation bar: it gives the way back to the app. */}
      <Stack.Screen
        options={{
          headerShown: true,
          title: "디자인 시스템",
          headerBackTitle: "앱으로",
          headerStyle: { backgroundColor: colors.backgroundBase },
          headerTintColor: colors.textPrimary,
          headerShadowVisible: false,
        }}
      />
      <ScrollView contentContainerStyle={styles.page}>{sections.slice(from - 1)}</ScrollView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  page: {
    padding: space[4],
    gap: space[8],
  },
  section: {
    gap: space[3],
  },
  row: {
    flexDirection: "row",
    flexWrap: "wrap",
    alignItems: "center",
    gap: space[2],
  },
  swatches: {
    flexDirection: "row",
    flexWrap: "wrap",
    rowGap: space[2],
  },
  swatch: {
    width: "50%",
    flexDirection: "row",
    alignItems: "center",
    gap: space[2],
  },
  swatchName: {
    flex: 1,
  },
  chip: {
    width: 28,
    height: 28,
    borderWidth: 1,
    borderRadius: radius.sm,
  },
  sample: {
    alignItems: "center",
    gap: space[1],
  },
  radius: {
    width: 48,
    height: 48,
    alignItems: "center",
    justifyContent: "center",
    borderWidth: 1,
  },
});

// The design system on a screen: every component in every variant and state. A development
// screen, reached by opening the app at /design. `/design?part=2` starts at the second half, for
// taking screenshots from a script, which cannot scroll.

import { size, space, typography } from "@malmoon/tokens";
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
import { useLocalSearchParams } from "expo-router";
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

export default function Design() {
  const colors = useColors();
  const [value, setValue] = useState("");
  const firstHalf = useLocalSearchParams<{ part?: string }>().part !== "2";
  return (
    <SafeAreaView style={{ flex: 1, backgroundColor: colors.backgroundBase }}>
      <ScrollView contentContainerStyle={styles.page}>
        <Text variant="display" heading>
          디자인 시스템
        </Text>

        {firstHalf && (
          <>
            <Section title="글자">
              {TEXT_VARIANTS.map((variant) => (
                <Text key={variant} variant={variant}>
                  {variant} · 회의 자막 Malmoon 123
                </Text>
              ))}
              <Text variant="detail" tone="secondary">
                본문 {typography.body.fontSize}/{typography.body.lineHeight}, 컨트롤 높이{" "}
                {size.touch.controlSm}/{size.touch.controlMd}/{size.touch.controlLg}
              </Text>
            </Section>

            <Section title="버튼">
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
            </Section>
          </>
        )}

        <Section title="입력">
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
        </Section>

        <Section title="배지">
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
        </Section>

        <Section title="배너">
          {BANNER_TONES.map((tone) => (
            <Banner key={tone} tone={tone} title={tone}>
              자막은 이 기기에만 저장됩니다.
            </Banner>
          ))}
        </Section>

        <Section title="카드">
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
        </Section>
      </ScrollView>
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
});

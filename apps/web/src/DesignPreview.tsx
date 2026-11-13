// The design preview: every token group and every component in every variant and state, at
// `/design`. The mobile app has the same screen at `/design`, and the desktop app opens it with
// `malmoon --design`.

import { colors, radius, size, space, typography } from "@malmoon/tokens";
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
} from "@malmoon/ui";
import { type ReactNode, useState } from "react";

type Theme = "light" | "dark";

const kebab = (name: string) => name.replace(/[A-Z0-9]+/g, (match) => `-${match.toLowerCase()}`);

/** Semantic colors by group. Component colors are shown by the components that use them. */
const COMPONENT_COLORS = /^(button|field|badge|banner)/;
const COLOR_GROUPS = Object.entries(
  Object.groupBy(
    Object.keys(colors.light).filter((name) => !COMPONENT_COLORS.test(name)),
    (name) => kebab(name).split("-")[0] ?? name,
  ),
);

const VARIANTS: ButtonVariant[] = ["primary", "secondary", "ghost", "danger"];
const SIZES: ControlSize[] = ["sm", "md", "lg"];
const BADGE_TONES: BadgeTone[] = ["neutral", "accent", "success", "warning", "danger"];
const BANNER_TONES: BannerTone[] = ["info", "success", "warning", "danger"];
const TEXT_VARIANTS = Object.keys(typography) as (keyof typeof typography)[];

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-4">
      <Text variant="title" as="h2">
        {title}
      </Text>
      {children}
    </section>
  );
}

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex flex-wrap items-center gap-3">
      <Text variant="detail" tone="secondary" as="span" className="w-24 shrink-0">
        {label}
      </Text>
      {children}
    </div>
  );
}

export function DesignPreview() {
  const [theme, setTheme] = useState(document.documentElement.dataset.theme as Theme);
  const [value, setValue] = useState("");
  const toggleTheme = () => {
    const next = theme === "dark" ? "light" : "dark";
    document.documentElement.dataset.theme = next;
    setTheme(next);
  };

  return (
    <main className="mx-auto flex max-w-4xl flex-col gap-12 p-6">
      <header className="flex items-center justify-between">
        <Text variant="display" as="h1">
          Malmoon 디자인 시스템
        </Text>
        <div className="flex items-center gap-3">
          <a className="text-mm-detail text-mm-text-accent underline" href="/">
            앱으로
          </a>
          <Button variant="ghost" size="sm" onPress={toggleTheme}>
            {theme === "dark" ? "밝은 화면" : "어두운 화면"}
          </Button>
        </div>
      </header>

      <Section title="색">
        {COLOR_GROUPS.map(([group, names]) => (
          <div key={group} className="flex flex-col gap-2">
            <Text variant="label" tone="secondary">
              {group}
            </Text>
            <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
              {(names ?? []).map((name) => (
                <div key={name} className="flex items-center gap-2">
                  <span
                    className="size-8 shrink-0 rounded-mm-sm border border-mm-border-default"
                    style={{ background: `var(--mm-color-${kebab(name)})` }}
                  />
                  <Text variant="footnote" tone="secondary" as="span" className="break-all">
                    {kebab(name)}
                  </Text>
                </div>
              ))}
            </div>
          </div>
        ))}
      </Section>

      <Section title="글자">
        {TEXT_VARIANTS.map((name) => (
          <Row key={name} label={`${kebab(name)} ${typography[name].fontSize}`}>
            <Text variant={kebab(name) as TextVariant} as="span">
              회의 자막을 실시간으로 Malmoon 123
            </Text>
          </Row>
        ))}
      </Section>

      <Section title="간격, 모서리, 크기">
        <Row label="space">
          {Object.entries(space).map(([name, px]) => (
            <span key={name} className="flex flex-col items-center gap-1">
              <span className="bg-mm-accent-default" style={{ width: px, height: px }} />
              <Text variant="footnote" tone="secondary" as="span">
                {name}
              </Text>
            </span>
          ))}
        </Row>
        <Row label="radius">
          {Object.entries(radius).map(([name, px]) => (
            <span
              key={name}
              className="flex size-12 items-center justify-center border border-mm-border-strong"
              style={{ borderRadius: px }}
            >
              <Text variant="footnote" tone="secondary" as="span">
                {name}
              </Text>
            </span>
          ))}
        </Row>
        <Row label="control">
          <Text variant="detail" tone="secondary" as="span">
            마우스 {size.pointer.controlSm}/{size.pointer.controlMd}/{size.pointer.controlLg}, 터치{" "}
            {size.touch.controlSm}/{size.touch.controlMd}/{size.touch.controlLg}. 이 화면은 입력
            장치에 맞는 쪽을 쓴다.
          </Text>
        </Row>
      </Section>

      <Section title="버튼">
        {VARIANTS.map((variant) => (
          <Row key={variant} label={variant}>
            {SIZES.map((buttonSize) => (
              <Button key={buttonSize} variant={variant} size={buttonSize}>
                자막 시작
              </Button>
            ))}
            <Button variant={variant} disabled>
              쓸 수 없음
            </Button>
            <Button variant={variant} loading>
              연결하는 중
            </Button>
          </Row>
        ))}
      </Section>

      <Section title="입력">
        <div className="grid gap-4 sm:grid-cols-2">
          <TextField
            label="이메일"
            placeholder="name@example.com"
            description="로그인 코드를 보낼 주소입니다."
            value={value}
            onChange={setValue}
          />
          <TextField label="회의 이름" value="주간 제품 회의" onChange={() => {}} required />
          <TextField
            label="로그인 코드"
            value="12345"
            onChange={() => {}}
            error="코드가 맞지 않거나 만료되었습니다."
          />
          <TextField label="조직" value="Malmoon" onChange={() => {}} disabled />
        </div>
      </Section>

      <Section title="배지">
        <Row label="tone">
          {BADGE_TONES.map((tone) => (
            <Badge key={tone} tone={tone}>
              {tone}
            </Badge>
          ))}
        </Row>
        <Row label="dot">
          <Badge tone="success" dot>
            연결됨
          </Badge>
          <Badge tone="warning" dot>
            연결하는 중
          </Badge>
          <Badge tone="danger" dot>
            연결 끊김
          </Badge>
        </Row>
      </Section>

      <Section title="배너">
        {BANNER_TONES.map((tone) => (
          <Banner key={tone} tone={tone} title={tone}>
            자막은 이 기기에만 저장됩니다. 회의가 끝나면 요약을 만들 수 있습니다.
          </Banner>
        ))}
      </Section>

      <Section title="카드, 구분선, 스피너">
        <Card
          title="실시간 자막"
          description="게이트웨이가 보내는 자막을 표시합니다."
          actions={
            <Badge tone="success" dot>
              연결됨
            </Badge>
          }
        >
          <Text tone="secondary">카드는 관련된 내용을 하나로 묶는다.</Text>
          <Divider />
          <Row label="spinner">
            <Spinner size="sm" />
            <Spinner size="md" />
            <Spinner size="lg" label="불러오는 중" />
          </Row>
        </Card>
      </Section>
    </main>
  );
}

# Malmoon

여러 언어가 오가는 회의에서 듣고, 말하고, 마무리하는 것까지 돕는 실시간 회의 코파일럿.

지금은 기획과 기술 검증 단계다. 이 저장소에는 시장 분석, 제품 기획, 아키텍처 설계 문서와 함께 네 플랫폼의 뼈대 코드가 있다. 뼈대는 게이트웨이가 보내는 데모 자막을 데스크톱, 웹, 모바일이 받아 표시하는 데까지 이어져 있고, 음성 인식과 오디오 캡처는 아직 없다.

## 문서

| 문서 | 내용 |
| --- | --- |
| [Smooth 분석](docs/01-smooth-analysis.md) | 기준으로 삼은 경쟁 제품의 기능 구조, 기술 구성, 강점과 제약 |
| [시장과 미충족 니즈](docs/02-market-and-needs.md) | 경쟁 구도, 플랫폼 내장 기능의 위협, 아직 풀리지 않은 사용자 문제 |
| [제품 기획](docs/03-product-plan.md) | 제품 정의, 타깃, 기능, 요금제 가설, 로드맵, 리스크 |
| [시스템 아키텍처](docs/04-architecture.md) | 클라이언트 구성, 실시간 파이프라인, 데이터 경로, 원가 |
| [음성 인식 모델 비교](docs/05-stt-comparison.md) | 실시간 전사 후보의 특징, 성능, 비용 |
| [디자인 시스템 아키텍처](docs/06-design-system.md) | 토큰 하나에서 웹, 데스크톱, 모바일의 코드를 생성하는 방법 |
| [출처](docs/sources.md) | 조사에 쓴 자료와 확인하지 못한 항목 |

## 핵심 결정

- **무엇을 만드나.** 봇 없이 시스템 오디오를 듣는 데스크톱 프롬프터다. 실시간 자막, 답변 제안, 음성 통역, 회의 후 요약과 코칭을 한 흐름으로 묶는다.
- **어디서 차별화하나.** 영어가 아니거나 언어가 섞이는 회의, 말하기 지원, 회사가 승인할 수 있는 데이터 경로다.
- **누구부터인가.** 한국어 사용자와 비한국어 사용자가 섞인 5~50인 팀이다. 이 수요는 아직 가정이고 인터뷰로 먼저 확인한다.
- **언제인가.** 엔지니어 4명 기준으로 16주에 동등 수준 알파, 36주에 팀 유료 출시가 목표다.

## 기술 스택

| 영역 | 선택 |
| --- | --- |
| 데스크톱 | Rust, GPUI |
| 웹 | React 19, Vite, Tailwind, HeroUI |
| 모바일 | React Native (Expo) |
| 공유 코어와 게이트웨이 | Rust (axum, tokio) |
| 디자인 시스템 | 자체 DTCG 토큰에서 CSS, TypeScript, Rust 코드를 생성 |
| 음성 인식 | Soniox (예비 ElevenLabs) |
| 언어 모델 | Claude Haiku 5.5, Claude Sonnet 5.5 |
| 도구 | mise, Cargo 워크스페이스, pnpm 워크스페이스, TypeScript 7.1(네이티브 컴파일러), Biome |

## 개발

도구 버전과 작업은 [mise](https://mise.jdx.dev)가 관리한다. mise만 설치하면 Rust, Node, pnpm은 고정된 버전으로 따라온다.

```sh
mise install        # 도구 설치
mise run check      # CI가 하는 검사 전부
mise tasks --all    # 작업 목록
```

자막이 흐르는 것을 보려면 게이트웨이를 먼저 띄우고 클라이언트를 실행한다.

```sh
mise run //gateway:dev    # 데모 자막을 보내는 게이트웨이 (127.0.0.1:8787)
mise run //desktop:dev    # 데스크톱 프롬프터 오버레이 (macOS)
mise run //web:dev        # 웹 앱 (http://localhost:5173)
mise run //mobile:dev     # Expo 개발 서버
```

| 경로 | 내용 |
| --- | --- |
| `apps/desktop` | GPUI 프롬프터 오버레이 |
| `apps/gateway` | 실시간 게이트웨이. 지금은 세션 인사와 데모 자막만 있다 |
| `apps/web` | React 웹 앱 |
| `apps/mobile` | Expo 모바일 앱 |
| `crates/malmoon-core` | 세션 프로토콜, 자막 안정화, 어절 단위 줄바꿈 |
| `crates/malmoon-tokens` | 생성된 Rust 토큰 |
| `crates/malmoon-ui` | 토큰을 입힌 GPUI 컴포넌트 |
| `packages/tokens` | 생성된 CSS, Tailwind 테마, TypeScript 토큰 |
| `packages/session` | 생성된 프로토콜 타입과 세션 클라이언트 |
| `design/tokens` | 디자인 토큰의 원천 |
| `tools/design-build` | 토큰 검증과 생성기 |

디자인 토큰이나 프로토콜 타입을 고친 뒤에는 `mise run generate`를 실행하고 생성된 파일을 함께 커밋한다. CI가 생성물이 최신인지 확인한다.

## 다음 단계

0단계(4주)에서 두 가지를 확인한다.

1. **수요.** Smooth 유료 사용자 10명과 혼합 언어 팀 10곳을 인터뷰한다.
2. **기술.** GPUI 오버레이, 캡처부터 자막까지의 지연, 한영 혼용 인식 품질, 토큰 생성기를 시제품으로 검증한다. macOS의 GPUI 오버레이와 토큰 생성기는 확인했고, 나머지는 남아 있다. 결과는 [시스템 아키텍처](docs/04-architecture.md)에 있다.

# 출처

> 모든 자료는 2026-11-12와 13에 확인했다. 가격과 버전은 이후 달라질 수 있다.

## 조사 방법

- 공개 웹 페이지, 공식 문서, 공개 저장소의 소스를 읽었다.
- 유료 서비스에 가입하거나 제품을 실행해 측정하지는 않았다. 예외로 GPUI 오버레이는 시제품을 만들어 실행했다.
- 본문에서 미확인이나 가정이라고 적은 항목은 근거를 찾지 못했거나 검색 요약으로만 본 것이다.

## Smooth

- [한국어 랜딩 페이지와 FAQ](https://www.trysmooth.ai/ko)
- [팀 플랜](https://www.trysmooth.ai/ko/team), [모바일](https://www.trysmooth.ai/ko/mobile), [릴리스 노트](https://www.trysmooth.ai/release-notes)
- [개인정보처리방침](https://www.trysmooth.ai/privacy), [이용약관](https://www.trysmooth.ai/terms-of-service), [생체정보 방침](https://www.trysmooth.ai/biometric-policy)
- [App Store](https://apps.apple.com/us/app/smooth-ai-english-meeting-ai/id6791071942), [Google Play](https://play.google.com/store/apps/details?id=ai.trysmooth.app)
- [긱뉴스 출시 글](https://news.hada.io/topic?id=26389)
- [ARR Club 기사](https://www.arr.club/smooth-ai/smooth-ai-crossed-300k-arr-in-8-months-with-65-mrr-growth-in-two-months) (창업자 게시물을 인용한 2차 자료)

## 경쟁 제품

- [JotMe](https://jotme.io), [Transync AI](https://transyncai.com), [Palabra](https://palabra.ai), [DeepL Voice](https://www.deepl.com/products/voice), [Cluely](https://cluely.com), [Tiro](https://tiro.ooo)
- Microsoft Teams, Google Meet, Zoom의 내장 기능은 각 사 공식 문서

## 사용자 근거와 연구

- [CSCW 2026 논문: 비원어민의 말하기 불안과 AI 지원](https://arxiv.org/abs/2604.18171)
- [HiKE: 한영 혼용 음성 인식 벤치마크](https://arxiv.org/abs/2509.24613)
- [Blind 글](https://www.teamblind.com/post/i-struggle-in-meetings-with-india-colleagues-j8ywowzr)
- 브런치 글 [1](https://brunch.co.kr/@loiskim01/422), [2](https://brunch.co.kr/@kk2daddy/283)
- [UC Today 기사](https://www.uctoday.com/unified-communications/live-translation-in-uc/), [tl;dv의 Cluely 리뷰](https://tldv.io/blog/cluely-review/)

## 음성 인식과 실시간 모델

[음성 인식 모델 비교](05-stt-comparison.md#출처) 문서 끝에 벤더별 링크가 있다.

- [Artificial Analysis 스트리밍 순위](https://artificialanalysis.ai/speech-to-text/streaming)
- [Pipecat STT 벤치마크](https://github.com/pipecat-ai/stt-benchmark)

## 데스크톱 (GPUI)

- [zed-industries/zed](https://github.com/zed-industries/zed) 저장소의 `crates/gpui*` 소스 (2026-11-12 main)
- [longbridge/gpui-kit](https://github.com/longbridge/gpui-kit) (gpui-component)
- [zed-industries/awesome-gpui](https://github.com/zed-industries/awesome-gpui)
- [rust-windowing/winit](https://github.com/rust-windowing/winit) 저장소의 macOS 창 소스 (v0.30.13과 2026-11-11 master). Iced, Slint, egui가 쓰는 창 계층이다
- macOS 26.5에서 직접 만든 GPUI 오버레이 시제품의 실행 결과. 한국어 입력은 자동 입력 시험으로, 블러 배경은 화면 캡처로 확인했다

## 디자인 시스템

- [meta/astryx](https://github.com/meta/astryx) 저장소 (커밋 `55b9032`): `packages/native`, `packages/core/src/theme`, `docs/architecture`, `docs/specs`. 검토하고 택하지 않았다
- [HeroUI](https://heroui.com/docs), [HeroUI Native](https://heroui.com/docs/native), [Uniwind](https://uniwind.dev/). 패키지 라이선스는 npm 레지스트리에서 확인했다
- [shadcn/ui](https://ui.shadcn.com/), [React Native Reusables](https://github.com/founded-labs/react-native-reusables), [Tamagui](https://tamagui.dev/), [gluestack-ui](https://gluestack.io/)
- [DTCG 디자인 토큰 형식 2025.10](https://www.w3.org/community/reports/design-tokens/CG-FINAL-format-20251028/)
- [Terrazzo](https://terrazzo.app/docs/), [Style Dictionary](https://styledictionary.com/)
- [Pretendard](https://github.com/orioncactus/pretendard)
- [React Native 텍스트 문서](https://reactnative.dev/docs/text)

## 확인하지 못한 것

- Smooth에 대한 독립적인 사용자 후기와 실제 지연 시간
- 한국어와 한영 혼용 음성 인식의 독립 벤치마크
- Google Meet 음성 번역의 한국어 포함 여부
- GPUI 오버레이가 다른 앱의 전체 화면 위에 실제로 표시되는지 (창 설정값은 실행해 확인했고, 전체 화면 회의 앱 위에서는 보지 않았다)
- GPUI 오버레이의 Windows 동작
- HeroUI와 HeroUI Native가 같은 테마 파일과 같은 컴포넌트 API를 쓰는지
- HeroUI Pro와 Uniwind Pro의 유료 범위
- 모바일 앱의 기기 실행 (번들링까지만 확인했다)
- Iced, Slint, egui의 글자 품질과 성능 (문서와 소스만 읽었다)

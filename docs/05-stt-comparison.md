# 실시간 음성 인식 모델 비교

> 기준일 2026-11-12. 가격과 사양은 각 사 공식 페이지에서 확인했다.
> 범위는 오디오를 받아 전사를 내는 구간까지다. 번역과 음성 합성은 [아키텍처 문서](04-architecture.md)에서 다룬다.

## 판단

기본 엔진은 **Soniox stt-rt-v6**, 예비는 **ElevenLabs Scribe v3 Live**다.

- **Soniox.** 두 채널 회의 1시간에 $0.24로 가장 싸다. 한영 문장 내 전환, 실시간 화자 구분, 번역이 한 소켓에 들어 있다.
- **ElevenLabs.** 독립 영어 벤치마크에서 오류율이 가장 낮다(2.86%). 가격은 Soniox의 1.8배다.
- **Deepgram.** Nova-3 다국어 모드에 한국어가 들어 있지만 Soniox의 4.6배 가격이다. Flux는 한국어를 지원하지 않는다.
- **Google.** Gemini 3.5 Transcribe Live는 한국어와 한영 혼용을 지원하고 정확도도 상위권이다. 다만 Soniox의 4.5배 가격이고, 실시간 화자 구분이 없으며 세션이 10분마다 끊긴다. 3순위 후보로 둔다.
- **OpenAI.** 전사 용도로는 Soniox보다 3배 이상 비싸고 확정이 느리다. 화자 라벨과 타임스탬프도 없다. 음성 통역 쪽 후보로만 둔다.

한국어와 한영 혼용 정확도는 어느 후보도 독립 수치가 없다.
0단계에서 자체 평가 세트로 Soniox와 ElevenLabs를 직접 비교해야 확정할 수 있다.

## 특징

| 모델 | 한국어 | 한영 혼용 | 실시간 화자 구분 | 용어 지원 | 발화 종료 감지 | 그 밖의 특징과 제한 |
| --- | --- | --- | --- | --- | --- | --- |
| Soniox stt-rt-v6 | 지원 (60개 이상 언어) | 토큰 단위 언어 식별, 문장 내 전환 | 최대 15명. 비동기보다 정확도가 낮다 | 문맥과 용어 목록 | 의미 기반 감지 | 번역 내장. 스트림당 300분, 동시 10개(상향 가능) |
| ElevenLabs Scribe v3 Live | 지원 (90개 이상 언어) | 다국어 모드로 문장 내 전환 | 부분 결과에 초안 라벨, 확정 후 별도 전사 | 키텀 50개와 용어 라이브러리 1만 개(유료) | 턴 예측 | 일부 기능은 공식 SDK에 없어 웹소켓을 직접 써야 한다 |
| Deepgram Nova-3 | 지원 (단일 언어, 다국어 모드 55개 언어에도 포함) | 다국어 모드가 지원한다고 하나 한영 전환은 미확인 | 유료 추가 기능 | 키텀 100개(유료) | 엔드포인팅 | 동시 150개. 자체 호스팅 가능 |
| Deepgram Flux | 미지원 (영어와 10개 언어) | 해당 없음 | 문서가 서로 다름 | 키텀 | 턴 감지 내장 | 음성 에이전트용. 자사 문서가 회의 전사에는 맞지 않는다고 적는다 |
| Google Gemini 3.5 Transcribe Live | 지원 (85개 이상 언어) | 자동 감지, 세션 중 혼용 지원 | 없음 (배치 전용) | 용어 1,000개 | 자동, 혼합, 수동 VAD | 세션 10분 상한이라 1시간에 6번 재연결한다. 단어 타임스탬프 없음 |
| Google Chirp 3 스트리밍 | 지원 (ko-KR 정식) | 가장 많이 쓰인 언어 하나만 추정 | 없음 (배치 전용) | 편향 구문과 프롬프트 | 음성 활동 이벤트 | 스트림 약 5분 상한. 단어 타임스탬프 없음 |
| OpenAI gpt-live-transcribe | 명시 없음 (언어 힌트만) | 문서 없음 | 없음 | 키워드와 프롬프트 | 없음. 클라이언트가 구간을 끊어야 한다 | 단어 타임스탬프와 신뢰도가 없다 |
| OpenAI gpt-realtime-2.1-mini | 음성 대화 모델 | 문서 없음 | 없음 | 프롬프트 | 서버 VAD, 의미 VAD | 전사 전용이 아니다. 세션 60분 상한 |
| OpenAI gpt-realtime-translate | 지원 | 문서 없음 | 없음 | 미확인 | 미확인 | 원문 전사와 번역 음성을 함께 낸다 |

## 성능 지표

독립 벤치마크는 모두 영어 기준이다. 두 벤치마크는 데이터와 측정 정의가 달라서 절대값이 아니라 순위로 읽어야 한다.
표는 [Artificial Analysis](https://artificialanalysis.ai/speech-to-text/streaming)(AA) 오류율이 낮은 순이다.

| 모델 | AA 오류율 | AA 첫 부분 결과 | AA 확정 지연 | [Pipecat](https://github.com/pipecat-ai/stt-benchmark) 오류율 | Pipecat 확정 지연 (중앙값) |
| --- | --- | --- | --- | --- | --- |
| ElevenLabs Scribe v3 Live | 2.86% | 0.072초 | 0.081초 | 미등재 | 미등재 |
| ElevenLabs Scribe v2 Realtime | 3.59% | 미확인 | 0.141초 | 3.13% | 281ms |
| OpenAI gpt-live-transcribe | 3.92% | 0.256초 | 0.81초 | 미등재 | 미등재 |
| Google Gemini 3.5 Transcribe Live | 4.00% | 0.250초 | 0.395초 | 1.97% | 458ms |
| Soniox stt-rt-v6 | 4.57% | 0.061초 | 0.080초 | 0.71% | 266ms |
| Google Chirp 3 스트리밍 | 4.80% | 1.200초 | 1.276초 | 미등재 | 미등재 |
| OpenAI gpt-realtime-whisper | 4.89% | 0.206초 | 0.688초 | 2.30% | 740ms |
| Deepgram Flux (영어) | 5.91% | 0.040초 | 0.042초 | 미등재 | 미등재 |
| Deepgram Nova-3 | 6.59% | 미확인 | 0.066초 | 1.37% | 247ms |

- **지연은 변별력이 없다.** Soniox, ElevenLabs, Deepgram 모두 첫 부분 결과가 0.1초 안, 확정이 0.3초 안이다. Gemini 3.5 Transcribe Live는 확정 0.4초로 그다음이고, OpenAI 계열(0.7~0.8초)과 Chirp 3(1.3초)는 느리다.
- **벤더 발표 수치.** ElevenLabs v2 Realtime은 확정 전사까지 중앙값 약 150ms(서버 측)다. Deepgram Nova-3 스트리밍 오류율 중앙값은 6.84%(자사 데이터)다. Soniox는 지연 수치를 발표하지 않는다.
- **Google 발표 수치.** 26개 언어 FLEURS 스트리밍 오류율이 Gemini 3.5 Transcribe Live 5.50%, Chirp 3 7.32%다. 자사 측정이다.
- **한국어 수치는 사실상 없다.** ElevenLabs가 자사 표에서 한국어를 오류율 10~20% 구간으로 분류한 것이 유일하고, 배치 처리와 v2 기준이다.

## 비용

회의 1시간에 마이크와 시스템 오디오 두 채널을 60분씩 보내는 경우로 통일했다.

| 모델 | 과금 기준 | 오디오 1시간 (USD) | 회의 1시간, 2채널 (USD) | 부가 요금과 조건 |
| --- | --- | --- | --- | --- |
| Soniox stt-rt-v6 | 토큰 (오디오 길이와 출력 텍스트) | 0.12 | 0.24 | 화자 구분 포함. 번역을 켜면 채널당 약 0.06 추가(추정) |
| ElevenLabs Scribe v3 Live | 시간 | 0.22 | 0.44 | 키텀 시간당 0.08. 화자 구분 요금은 미확인 |
| OpenAI gpt-realtime-2.1-mini (청취 전용) | 토큰 | 0.40 | 0.79 | 하한 추정. 문맥 재전송 비용은 빠져 있다 |
| Google Gemini 3.8 Live (청취 전용) | 토큰 | 0.34 이상 | 0.68 이상 | 하한 추정. 텍스트만 출력할 수 없고 모델이 말하면 비용이 는다 |
| Deepgram Nova-3 단일 언어 | 초 | 0.46 | 0.92 | 화자 구분 시간당 0.12, 키텀 시간당 0.078 |
| Deepgram Flux | 초 | 0.46~0.47 | 0.92~0.94 | 한국어 미지원 |
| Google Gemini 3.5 Transcribe Live | 토큰 | 0.54 | 1.07 | 화자 구분 없음. 음성 입력 초당 25토큰 |
| Deepgram Nova-3 다국어 | 초 | 0.55 | 1.10 | 화자 구분을 더하면 1.34 |
| Google Chirp 3 스트리밍 | 초 | 0.96 | 1.92 | 월 50만 분 이상은 단가 인하 |
| OpenAI gpt-live-transcribe | 분 | 1.02 | 2.04 | 없음 |
| OpenAI gpt-realtime-2.1 (청취 전용) | 토큰 | 1.51 | 3.02 | 하한 추정 |
| OpenAI gpt-realtime-translate | 분 | 2.04 | 4.08 | 번역 음성 포함 |

월 40시간 회의하는 사용자라면 Soniox $9.6, ElevenLabs $17.6, Gemini 3.5 Transcribe Live $43, Deepgram 다국어 $44, OpenAI gpt-live-transcribe $82다.
개인 요금제 월 $19 가설에서는 Soniox만 여유가 있다.

### OpenAI 실시간 모델의 토큰 단가

100만 토큰당 USD다. 음성 입력은 초당 10토큰(시간당 36,000), 음성 출력은 초당 20토큰으로 계산된다.

| 모델 | 음성 입력 | 음성 출력 | 텍스트 입력 | 텍스트 출력 | 음성 출력 1분 |
| --- | --- | --- | --- | --- | --- |
| gpt-realtime-2.1 | 32 | 64 | 4 | 24 | $0.077 |
| gpt-realtime-2.1-mini | 10 | 20 | 0.60 | 2.40 | $0.024 |

청취 전용 비용은 시간당 음성 입력 36,000토큰과 텍스트 출력 15,000토큰을 가정해 환산했다.
실제로는 응답마다 대화 문맥을 다시 보내므로 이보다 더 든다.

## 확인하지 못한 항목

- 한국어와 한영 혼용 정확도 (모든 후보)
- Soniox의 벤더 지연 수치와 무보존 옵션
- ElevenLabs 실시간 화자 구분의 요금과 과금 기준
- Deepgram Flux의 화자 구분 지원 여부 (자사 문서가 서로 다름)
- OpenAI gpt-live-transcribe의 한국어 지원 명시
- Gemini 3.5 Transcribe Live의 출시 상태 (Vertex는 정식, 출시 블로그는 프리뷰로 적는다)와 동시 접속 한도

## 출처

- Soniox: [가격](https://soniox.com/pricing), [모델](https://docs.soniox.com/stt/models), [제한](https://docs.soniox.com/stt/rt/limits-and-quotas)
- Deepgram: [가격](https://deepgram.com/pricing), [Nova-3 언어](https://developers.deepgram.com/docs/nova-3-language-prompting.md), [Flux 언어](https://developers.deepgram.com/docs/flux/language-prompting.md), [Flux와 Nova-3 비교](https://developers.deepgram.com/docs/flux/flux-nova-3-comparison.md)
- ElevenLabs: [API 가격](https://elevenlabs.io/pricing/api), [모델](https://elevenlabs.io/docs/overview/models.md), [지연 측정 방법](https://elevenlabs.io/docs/eleven-api/concepts/latency-methodology.md)
- Google: [Gemini API 가격](https://ai.google.dev/gemini-api/docs/pricing), [Transcribe Live 문서](https://ai.google.dev/gemini-api/docs/live-api/live-transcribe), [Chirp 3](https://docs.cloud.google.com/speech-to-text/docs/models/chirp-3), [Speech-to-Text 가격](https://cloud.google.com/speech-to-text/pricing)
- OpenAI: [가격](https://developers.openai.com/api/docs/pricing.md), [실시간 전사 가이드](https://developers.openai.com/api/docs/guides/realtime-transcription.md), [음성 지연과 비용 가이드](https://developers.openai.com/api/docs/guides/voice-latency-cost.md)

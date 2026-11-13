//! A scripted caption feed for developing clients before a speech model is connected.
//!
//! It imitates what real recognition looks like on the wire: each segment grows word by word as a
//! draft, settles as a final, and then gets its translation refined.

use std::time::Duration;

use malmoon_core::protocol::{CaptionStage, CaptionUpdate, Channel, ServerMessage};
use tokio::sync::mpsc;

const WORD_INTERVAL: Duration = Duration::from_millis(180);
const SENTENCE_PAUSE: Duration = Duration::from_millis(1200);

const SCRIPT: &[(Channel, &str, &str)] = &[
    (
        Channel::System,
        "Thanks for joining, let's start with the launch timeline.",
        "참석해 주셔서 감사합니다. 출시 일정부터 시작하겠습니다.",
    ),
    (
        Channel::System,
        "Can the Korean team confirm the beta date by Friday?",
        "한국 팀에서 금요일까지 베타 날짜를 확정해 주실 수 있나요?",
    ),
    (
        Channel::Mic,
        "네, 금요일 오전까지 확정해서 공유드리겠습니다.",
        "Yes, we will confirm and share it by Friday morning.",
    ),
    (
        Channel::System,
        "Great. Then the only open risk is the audio capture on Windows.",
        "좋습니다. 그러면 남은 위험은 Windows 오디오 캡처뿐입니다.",
    ),
];

/// Sends the script in a loop until the receiver is dropped.
pub(crate) async fn run(outbound: mpsc::Sender<ServerMessage>) {
    let mut segment_id = 0u64;
    loop {
        for &(channel, original, translation) in SCRIPT {
            segment_id += 1;
            let caption =
                |stage, original: String, translation: Option<&str>| ServerMessage::Caption {
                    caption: CaptionUpdate {
                        segment_id,
                        channel,
                        stage,
                        original,
                        translation: translation.map(str::to_owned),
                    },
                };

            let mut spoken = String::new();
            for word in original.split(' ') {
                if !spoken.is_empty() {
                    spoken.push(' ');
                }
                spoken.push_str(word);
                let draft = caption(CaptionStage::Draft, spoken.clone(), None);
                if outbound.send(draft).await.is_err() {
                    return;
                }
                tokio::time::sleep(WORD_INTERVAL).await;
            }

            let settled = caption(CaptionStage::Final, original.to_owned(), None);
            if outbound.send(settled).await.is_err() {
                return;
            }
            tokio::time::sleep(WORD_INTERVAL * 2).await;

            let refined = caption(CaptionStage::Final, original.to_owned(), Some(translation));
            if outbound.send(refined).await.is_err() {
                return;
            }
            tokio::time::sleep(SENTENCE_PAUSE).await;
        }
    }
}

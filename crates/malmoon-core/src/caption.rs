//! Turns a stream of caption updates into a stable list of lines to display.
//!
//! Recognition revises a segment many times before it settles. The buffer keeps one entry per
//! segment, replaces it in place on every update, and drops the oldest settled segments once it
//! holds more than it was asked to keep.

use crate::protocol::{CaptionStage, CaptionUpdate};

#[derive(Debug, Clone)]
pub struct CaptionBuffer {
    max_final: usize,
    segments: Vec<CaptionUpdate>,
}

impl CaptionBuffer {
    /// Keeps at most `max_final` settled segments. Draft segments are always kept.
    pub fn new(max_final: usize) -> Self {
        Self {
            max_final,
            segments: Vec::new(),
        }
    }

    /// Applies one update. Returns `false` if the update was stale and changed nothing.
    pub fn apply(&mut self, update: CaptionUpdate) -> bool {
        match self
            .segments
            .binary_search_by_key(&update.segment_id, |segment| segment.segment_id)
        {
            Ok(index) => {
                let current = &mut self.segments[index];
                // A draft that arrives after its segment settled is out of order: ignore it.
                if current.stage == CaptionStage::Final && update.stage == CaptionStage::Draft {
                    return false;
                }
                if *current == update {
                    return false;
                }
                *current = update;
            }
            Err(index) => {
                // A settled segment older than everything kept has already scrolled away.
                if update.stage == CaptionStage::Final
                    && index == 0
                    && self.final_count() >= self.max_final
                {
                    return false;
                }
                self.segments.insert(index, update);
            }
        }
        self.evict();
        true
    }

    /// Segments in speaking order, oldest first.
    pub fn segments(&self) -> &[CaptionUpdate] {
        &self.segments
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    pub fn clear(&mut self) {
        self.segments.clear();
    }

    fn final_count(&self) -> usize {
        self.segments
            .iter()
            .filter(|segment| segment.stage == CaptionStage::Final)
            .count()
    }

    fn evict(&mut self) {
        let mut excess = self.final_count().saturating_sub(self.max_final);
        self.segments.retain(|segment| {
            if excess > 0 && segment.stage == CaptionStage::Final {
                excess -= 1;
                false
            } else {
                true
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Channel;

    fn update(segment_id: u64, stage: CaptionStage, original: &str) -> CaptionUpdate {
        CaptionUpdate {
            segment_id,
            channel: Channel::System,
            stage,
            original: original.into(),
            translation: None,
        }
    }

    fn texts(buffer: &CaptionBuffer) -> Vec<&str> {
        buffer
            .segments()
            .iter()
            .map(|segment| segment.original.as_str())
            .collect()
    }

    #[test]
    fn a_draft_is_replaced_in_place() {
        let mut buffer = CaptionBuffer::new(3);
        assert!(buffer.apply(update(1, CaptionStage::Draft, "Let's")));
        assert!(buffer.apply(update(1, CaptionStage::Draft, "Let's ship")));
        assert!(buffer.apply(update(1, CaptionStage::Final, "Let's ship it.")));
        assert_eq!(texts(&buffer), ["Let's ship it."]);
    }

    #[test]
    fn a_late_draft_does_not_undo_a_final() {
        let mut buffer = CaptionBuffer::new(3);
        buffer.apply(update(1, CaptionStage::Final, "Done."));
        assert!(!buffer.apply(update(1, CaptionStage::Draft, "Don")));
        assert_eq!(texts(&buffer), ["Done."]);
    }

    #[test]
    fn a_final_can_be_refined() {
        let mut buffer = CaptionBuffer::new(3);
        buffer.apply(update(1, CaptionStage::Final, "Done."));
        let mut refined = update(1, CaptionStage::Final, "Done.");
        refined.translation = Some("끝났습니다.".into());
        assert!(buffer.apply(refined.clone()));
        assert!(
            !buffer.apply(refined),
            "an identical update changes nothing"
        );
        assert_eq!(
            buffer.segments()[0].translation.as_deref(),
            Some("끝났습니다.")
        );
    }

    #[test]
    fn the_oldest_finals_are_dropped_and_drafts_survive() {
        let mut buffer = CaptionBuffer::new(2);
        buffer.apply(update(1, CaptionStage::Final, "one"));
        buffer.apply(update(2, CaptionStage::Draft, "two"));
        buffer.apply(update(3, CaptionStage::Final, "three"));
        buffer.apply(update(4, CaptionStage::Final, "four"));
        assert_eq!(texts(&buffer), ["two", "three", "four"]);
    }

    #[test]
    fn segments_stay_in_speaking_order_when_updates_interleave() {
        let mut buffer = CaptionBuffer::new(5);
        buffer.apply(update(2, CaptionStage::Draft, "second"));
        buffer.apply(update(1, CaptionStage::Draft, "first"));
        assert_eq!(texts(&buffer), ["first", "second"]);
    }

    #[test]
    fn a_segment_older_than_a_full_buffer_is_ignored() {
        let mut buffer = CaptionBuffer::new(1);
        buffer.apply(update(5, CaptionStage::Final, "five"));
        assert!(!buffer.apply(update(2, CaptionStage::Final, "two")));
        assert_eq!(texts(&buffer), ["five"]);
    }
}

pub const fn max_offset(total_items: usize, visible_items: usize) -> usize {
    total_items.saturating_sub(visible_items)
}

/// ratatui's scrollbar position spans `0..content_length - 1`. A scrolling
/// viewport therefore has max_offset + 1 positions, not total_items positions.
pub const fn position_count(total_items: usize, visible_items: usize) -> usize {
    max_offset(total_items, visible_items) + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_scroll_offset_is_the_final_scrollbar_position() {
        let total = 50;
        let visible = 38;
        assert_eq!(max_offset(total, visible), 12);
        assert_eq!(position_count(total, visible), 13);
        assert_eq!(
            max_offset(total, visible),
            position_count(total, visible) - 1
        );
    }

    #[test]
    fn non_scrolling_content_has_one_stable_position() {
        assert_eq!(max_offset(4, 10), 0);
        assert_eq!(position_count(4, 10), 1);
    }
}

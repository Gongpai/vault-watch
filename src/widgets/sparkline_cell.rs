const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Convert a history slice (raw u64 values) into a Unicode sparkline string.
/// `width` is the number of characters to fill. If the slice is shorter, it is
/// right-aligned with spaces on the left.
pub fn sparkline(history: &[u64], width: usize) -> String {
    if history.is_empty() || width == 0 {
        return " ".repeat(width);
    }

    let max = *history.iter().max().unwrap_or(&1);
    let max = if max == 0 { 1 } else { max };

    // Take up to `width` most-recent samples
    let samples: Vec<u64> = if history.len() >= width {
        history[history.len() - width..].to_vec()
    } else {
        history.to_vec()
    };

    let mut result = String::with_capacity(width);

    // Pad left with spaces if we have fewer samples than width
    let padding = width.saturating_sub(samples.len());
    for _ in 0..padding {
        result.push(' ');
    }

    for &val in &samples {
        let idx = (val as f64 / max as f64 * 7.0).round() as usize;
        result.push(BLOCKS[idx.min(7)]);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        assert_eq!(sparkline(&[], 5), "     ");
    }

    #[test]
    fn test_all_zero() {
        let data = vec![0u64; 4];
        let s = sparkline(&data, 4);
        assert_eq!(s.chars().count(), 4);
        // All zero → all '▁' (index 0)
        assert!(s.chars().all(|c| c == '▁'));
    }

    #[test]
    fn test_padding() {
        let data = vec![10u64, 20];
        let s = sparkline(&data, 5);
        assert_eq!(s.chars().count(), 5);
        // First 3 chars are spaces
        let chars: Vec<char> = s.chars().collect();
        assert_eq!(chars[0], ' ');
        assert_eq!(chars[1], ' ');
        assert_eq!(chars[2], ' ');
    }

    #[test]
    fn test_max_block() {
        let data = vec![0u64, 100];
        let s = sparkline(&data, 2);
        let chars: Vec<char> = s.chars().collect();
        assert_eq!(chars[0], '▁');
        assert_eq!(chars[1], '█');
    }
}

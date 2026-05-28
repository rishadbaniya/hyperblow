use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct PiecePicker {
    piece_count: usize,
    completed: Vec<bool>,
    requested: Vec<bool>,
}

impl PiecePicker {
    pub fn new(piece_count: usize) -> Self {
        Self {
            piece_count,
            completed: vec![false; piece_count],
            requested: vec![false; piece_count],
        }
    }

    pub fn mark_completed(&mut self, piece_index: usize) {
        if piece_index < self.piece_count {
            self.completed[piece_index] = true;
            self.requested[piece_index] = false;
        }
    }

    pub fn mark_requested(&mut self, piece_index: usize) {
        if piece_index < self.piece_count && !self.completed[piece_index] {
            self.requested[piece_index] = true;
        }
    }

    pub fn mark_request_failed(&mut self, piece_index: usize) {
        if piece_index < self.piece_count && !self.completed[piece_index] {
            self.requested[piece_index] = false;
        }
    }

    pub fn next_rarest_piece(&self, peer_piece_sets: &[Vec<usize>]) -> Option<usize> {
        let mut availability = vec![0_usize; self.piece_count];
        for piece_set in peer_piece_sets {
            let mut counted_for_peer = HashSet::new();
            for &piece_index in piece_set {
                if piece_index < self.piece_count && counted_for_peer.insert(piece_index) {
                    availability[piece_index] += 1;
                }
            }
        }

        availability
            .iter()
            .enumerate()
            .filter(|(piece_index, count)| **count > 0 && !self.completed[*piece_index] && !self.requested[*piece_index])
            .min_by_key(|(piece_index, count)| (**count, *piece_index))
            .map(|(piece_index, _)| piece_index)
    }

    pub fn completed_count(&self) -> usize {
        self.completed.iter().filter(|&&completed| completed).count()
    }

    pub fn requested_count(&self) -> usize {
        self.requested.iter().filter(|&&requested| requested).count()
    }
}

#[cfg(test)]
mod tests {
    use super::PiecePicker;

    #[test]
    fn chooses_rarest_available_piece() {
        let picker = PiecePicker::new(5);
        let peers = vec![vec![0, 1, 2], vec![0, 2], vec![0, 3]];

        assert_eq!(picker.next_rarest_piece(&peers), Some(1));
    }

    #[test]
    fn ignores_completed_and_requested_pieces() {
        let mut picker = PiecePicker::new(5);
        picker.mark_completed(1);
        picker.mark_requested(3);
        let peers = vec![vec![1, 2, 3], vec![3, 4]];

        assert_eq!(picker.next_rarest_piece(&peers), Some(2));
    }

    #[test]
    fn failed_request_makes_piece_eligible_again() {
        let mut picker = PiecePicker::new(3);
        picker.mark_requested(2);
        assert_eq!(picker.next_rarest_piece(&[vec![2]]), None);

        picker.mark_request_failed(2);

        assert_eq!(picker.next_rarest_piece(&[vec![2]]), Some(2));
    }

    #[test]
    fn ignores_duplicate_and_out_of_range_peer_entries() {
        let picker = PiecePicker::new(3);
        let peers = vec![vec![0, 0, 99], vec![1, 2], vec![1]];

        assert_eq!(picker.next_rarest_piece(&peers), Some(0));
    }
}

use sha1::{Digest, Sha1};
use std::{collections::BTreeMap, ops::Range};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PieceAssemblyError {
    #[error("block starts past piece boundary: begin={begin}, piece_length={piece_length}")]
    BlockOutOfRange { begin: usize, piece_length: usize },

    #[error("block extends past piece boundary: end={end}, piece_length={piece_length}")]
    BlockTooLong { end: usize, piece_length: usize },

    #[error("block range overlaps previously inserted data")]
    OverlappingBlock,

    #[error("piece is incomplete")]
    Incomplete,

    #[error("assembled piece hash mismatch")]
    HashMismatch,
}

#[derive(Debug, Clone)]
pub struct PieceAssembler {
    expected_hash: [u8; 20],
    piece_length: usize,
    blocks: BTreeMap<usize, Vec<u8>>,
}

impl PieceAssembler {
    pub fn new(expected_hash: [u8; 20], piece_length: usize) -> Self {
        Self {
            expected_hash,
            piece_length,
            blocks: BTreeMap::new(),
        }
    }

    pub fn insert_block(&mut self, begin: usize, block: Vec<u8>) -> Result<(), PieceAssemblyError> {
        if begin >= self.piece_length {
            return Err(PieceAssemblyError::BlockOutOfRange {
                begin,
                piece_length: self.piece_length,
            });
        }

        let end = begin.saturating_add(block.len());
        if end > self.piece_length {
            return Err(PieceAssemblyError::BlockTooLong {
                end,
                piece_length: self.piece_length,
            });
        }

        let new_range = begin..end;
        if self.blocks.iter().any(|(&existing_begin, existing_block)| {
            ranges_overlap(new_range.clone(), existing_begin..existing_begin + existing_block.len())
        }) {
            return Err(PieceAssemblyError::OverlappingBlock);
        }

        self.blocks.insert(begin, block);
        Ok(())
    }

    pub fn is_complete(&self) -> bool {
        let mut offset = 0;
        for (&begin, block) in &self.blocks {
            if begin != offset {
                return false;
            }
            offset += block.len();
        }
        offset == self.piece_length
    }

    pub fn assemble(self) -> Result<Vec<u8>, PieceAssemblyError> {
        if !self.is_complete() {
            return Err(PieceAssemblyError::Incomplete);
        }

        let mut piece = vec![0_u8; self.piece_length];
        for (begin, block) in self.blocks {
            piece[begin..begin + block.len()].copy_from_slice(&block);
        }

        let actual_hash: [u8; 20] = Sha1::digest(&piece).into();
        if actual_hash != self.expected_hash {
            return Err(PieceAssemblyError::HashMismatch);
        }

        Ok(piece)
    }
}

fn ranges_overlap(left: Range<usize>, right: Range<usize>) -> bool {
    left.start < right.end && right.start < left.end
}

#[cfg(test)]
mod tests {
    use super::{PieceAssembler, PieceAssemblyError};
    use sha1::{Digest, Sha1};

    #[test]
    fn assembles_and_hash_validates_piece() {
        let piece = b"hello bittorrent piece".to_vec();
        let expected_hash: [u8; 20] = Sha1::digest(&piece).into();
        let mut assembler = PieceAssembler::new(expected_hash, piece.len());

        assembler.insert_block(6, piece[6..].to_vec()).expect("tail block");
        assembler.insert_block(0, piece[..6].to_vec()).expect("head block");

        assert_eq!(assembler.assemble().expect("piece should validate"), piece);
    }

    #[test]
    fn rejects_overlapping_blocks() {
        let mut assembler = PieceAssembler::new([0; 20], 10);
        assembler.insert_block(0, vec![1, 2, 3, 4]).expect("first block");

        assert_eq!(
            assembler.insert_block(3, vec![5, 6]).expect_err("overlap should fail"),
            PieceAssemblyError::OverlappingBlock
        );
    }

    #[test]
    fn rejects_incomplete_piece() {
        let mut assembler = PieceAssembler::new([0; 20], 10);
        assembler.insert_block(0, vec![1, 2, 3, 4]).expect("partial block");

        assert_eq!(
            assembler.assemble().expect_err("incomplete should fail"),
            PieceAssemblyError::Incomplete
        );
    }

    #[test]
    fn rejects_hash_mismatch() {
        let mut assembler = PieceAssembler::new([0; 20], 4);
        assembler.insert_block(0, vec![1, 2, 3, 4]).expect("complete block");

        assert_eq!(
            assembler.assemble().expect_err("hash mismatch should fail"),
            PieceAssemblyError::HashMismatch
        );
    }
}

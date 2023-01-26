use crate::core::peer::block::Block;
use bytes::{BufMut, BytesMut};

/// Holds all the raw data of a piece and the piece's metadata
pub struct Piece {
    /// Zero based index of the piece
    index: u32,

    /// Raw data of the piece
    data: BytesMut,

    /// Computed Hash of the piece
    hash: [u8; 20],
    //   /// Expected Hash of the piece
    //   info_hash: [u8; 20],

    //   /// Blocks of the piece
    //   blocks: Vec<Block>,
}

impl Piece {
    //   /// Creates a Piece from all the blocks provided
    //   /// We're gonna assume the blocks are in order
    //   pub fn from_blocks(blocks: Vec<Block>) -> Self {
    //       let mut data = BytesMut::new();

    //       //Takes one of the block from blocks and gets the piece index
    //       let index = blocks[0].piece_index;
    //       for block in blocks {
    //           data.put_slice(&block.raw_block);
    //       }

    //       // Get the sha1 hash of the piece data
    //       let mut hasher = Sha1::new();
    //       hasher.update(&data);
    //       let hash: [u8; 20] = hasher.finalize().into();

    //       Self { index, data, hash }
    //   }

    //   /// Checks the validity of the piece by tallying it with the hash provided as parameter, usually
    //   /// we take hash of the piece from the ".torrent" and then pass the hash here into the
    //   /// function and this function checks whether the hash mentioned in the ".torrent" file is
    //   /// equal to the computed hash of the piece data
    //   pub fn is_piece_valid(&self, hash: [u8; 20]) -> bool {
    //       if hash == self.hash {
    //           true
    //       } else {
    //           false
    //       }
    //   }

    //   /// TODO : Add implementation to sort the blocks
    //   fn sort_blocks(&self) {}
}

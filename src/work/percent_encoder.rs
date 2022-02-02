use percent_encoding::{percent_encode, NON_ALPHANUMERIC};

/// Encode the given byte vector of info_hash into a String of
/// Percent Encoded info_hash
pub fn encode(byteVector: Vec<u8>) -> String {
    percent_encode(&byteVector, NON_ALPHANUMERIC).to_string()
}

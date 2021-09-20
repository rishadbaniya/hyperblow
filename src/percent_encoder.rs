use percent_encoding::{percent_encode, NON_ALPHANUMERIC};

/// Encode the given byteVector of info_hash into a String of
/// URLEncoded(Percent Encoded) info_hash
pub fn encode(byteVector: Vec<u8>) -> String {
    percent_encode(&byteVector, NON_ALPHANUMERIC).to_string()
}

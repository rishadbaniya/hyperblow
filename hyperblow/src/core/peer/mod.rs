// Current state of the relationship with the Peer
#[derive(Debug, Clone)]
pub enum PeerState {
    /// Haven't even tried to connect
    NotConnected,
    /// Trying to connect
    TryingToConnect,
    /// Staying Idle, because Connection timeout occured
    ConnectionTimeoutIdle,
    /// Staying Idle, because some error occured while creating Connection
    ConnectionErrorIdle,
    /// Connected with the peer
    Connected,
    // TODO: Add more states later on
    //RequestingPiece,
    // DOWNLOADING_PIECE,
}

/// It defines the type of Peer
#[derive(Debug, Clone)]
pub enum PeerType {
    /// One that doesnt have all pieces and wants more piece
    Leecher,

    /// One that has all the needed pieces
    Seeder,

    /// One that has downloaded required files and doesnt wanna download other files
    PartialSeeder,

    /// PeerType not figured out
    Unknown,
}

/// PeerInfo holds crucial informations about the Peer, such as the pieces the peer has
/// or doesn't have, the type of the peer
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Zero based index of the piece the peer has
    pieces_have: Vec<u32>,
    /// Zero based index of the piece the peer does not have
    pieces_not_have: Vec<u32>,
    /// Whether the peer is a Seeder or Leecher
    peer_type: PeerType,
    /// State of the peer
    peer_state: PeerState,
}

#[derive(Debug)]
pub struct Peer {}

impl Peer {}

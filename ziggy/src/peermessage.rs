//! Data structures for a simple BitTorrent client

#[derive(Debug)]
pub enum ParseError {
    UnknownId(u8),
    InvalidPayload,
}

#[derive(Debug)]
pub enum PeerMessage {
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have(u32),
    Bitfield(Vec<u8>),
    Request {
        index: u32,
        begin: u32,
        length: u32,
    },
    Piece {
        index: u32,
        begin: u32,
        block: Vec<u8>,
    },
    Cancel {
        index: u32,
        begin: u32,
        length: u32,
    },
}

impl PeerMessage {
    pub fn id(&self) -> u8 {
        match self {
            PeerMessage::Choke => 0,
            PeerMessage::Unchoke => 1,
            PeerMessage::Interested => 2,
            PeerMessage::NotInterested => 3,
            PeerMessage::Have(_) => 4,
            PeerMessage::Bitfield(_) => 5,
            PeerMessage::Request { .. } => 6,
            PeerMessage::Piece { .. } => 7,
            PeerMessage::Cancel { .. } => 8,
        }
    }

    pub fn parse(id: u8, payload: &[u8]) -> Result<Self, ParseError> {
        Ok(match id {
            0 => PeerMessage::Choke,
            1 => PeerMessage::Unchoke,
            2 => PeerMessage::Interested,
            3 => PeerMessage::NotInterested,
            4 => {
                let payload = payload[0..4]
                    .try_into()
                    .map_err(|_| ParseError::InvalidPayload)?;
                PeerMessage::Have(u32::from_be_bytes(payload))
            }
            5 => PeerMessage::Bitfield(payload.to_vec()),
            6 => {
                let (index, begin, length) = parse_three_u32(payload)?;
                PeerMessage::Request {
                    index,
                    begin,
                    length,
                }
            }
            // ...
            other => return Err(ParseError::UnknownId(other)),
        })
    }
}

fn parse_three_u32(payload: &[u8]) -> Result<(u32, u32, u32), ParseError> {
    if payload.len() != 12 {
        return Err(ParseError::InvalidPayload);
    }
    let index = u32::from_be_bytes(
        payload[0..4]
            .try_into()
            .map_err(|_| ParseError::InvalidPayload)?,
    );
    let begin = u32::from_be_bytes(
        payload[4..8]
            .try_into()
            .map_err(|_| ParseError::InvalidPayload)?,
    );
    let length = u32::from_be_bytes(
        payload[8..12]
            .try_into()
            .map_err(|_| ParseError::InvalidPayload)?,
    );
    Ok((index, begin, length))
}

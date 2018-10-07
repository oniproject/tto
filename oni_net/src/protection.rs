#![allow(unused_variables)]
use crate::packet::{MIN_PACKET_BYTES, Kind, CHALLENGE_PACKET_BYTES};

pub const REPLAY_PROTECTION_BUFFER_SIZE: usize = 256;
const INVALID_SEQUENCE: u64 = 0xFFFF_FFFF_FFFF_FFFF;

pub trait Protection {
    fn packet_already_received(&mut self, seq: u64) -> bool {
        false
    }
    fn is_allowed(&mut self, kind: Kind, len: usize, seq: u64) -> bool {
        true
    }
}

pub struct NoFilter;
impl Protection for NoFilter {}

pub struct ChallengeFilter;
impl Protection for ChallengeFilter {
    fn is_allowed(&mut self, kind: Kind, len: usize, seq: u64) -> bool {
        if kind == Kind::Challenge && len == CHALLENGE_PACKET_BYTES {
            return true;
        }
        false
    }
}

pub struct ChallengeOrDisconnectFilter;
impl Protection for ChallengeOrDisconnectFilter {
    fn is_allowed(&mut self, kind: Kind, len: usize, seq: u64) -> bool {
        if kind == Kind::Challenge && len == CHALLENGE_PACKET_BYTES {
            return true;
        }
        if kind == Kind::Disconnect && len == MIN_PACKET_BYTES {
            return true;
        }
        false
    }
}

pub struct NoProtection;

impl Protection for NoProtection {
    fn packet_already_received(&mut self, _seq: u64) -> bool { false }
}

pub struct ReplayProtection {
    received_packet: Vec<u64>,
    most_recent_sequence: u64,
}

impl Default for ReplayProtection {
    fn default() -> Self {
        Self::new(REPLAY_PROTECTION_BUFFER_SIZE)
    }
}

impl ReplayProtection {
    pub fn new(len: usize) -> Self {
        Self {
            most_recent_sequence: 0,
            received_packet: vec![INVALID_SEQUENCE; len],
        }
    }

    pub fn most_recent_sequence(&self) -> u64 {
        self.most_recent_sequence
    }

    pub fn reset(&mut self) {
        self.most_recent_sequence = 0;
        self.received_packet = vec![INVALID_SEQUENCE; self.received_packet.len()];
    }
}

impl Protection for ReplayProtection {
    fn is_allowed(&mut self, kind: Kind, len: usize, seq: u64) -> bool {
        if kind == Kind::Request || kind == Kind::Challenge {
            return false;
        }
        if kind == Kind::Disconnect && len != MIN_PACKET_BYTES {
            return false;
        }
        !self.packet_already_received(seq)
    }

    fn packet_already_received(&mut self, sequence: u64) -> bool {
        if sequence + self.received_packet.len() as u64  <= self.most_recent_sequence as u64 {
            return true;
        }
        if sequence > self.most_recent_sequence {
            self.most_recent_sequence = sequence;
        }
        let index = (sequence % self.received_packet.len() as u64) as usize;
        if self.received_packet[index] == INVALID_SEQUENCE {
            self.received_packet[index] = sequence;
            return false;
        }
        if self.received_packet[index] >= sequence {
            return true
        }
        self.received_packet[index] = sequence;
        false
    }
}

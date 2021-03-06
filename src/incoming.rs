use std::{
    net::SocketAddr,
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use crate::{
    token::{
        ChallengeToken,
        PrivateToken,
        CHALLENGE_LEN,
    },
    crypto::{keygen, KEY, HMAC},
    protocol::{Packet, Request},
    unix_time,
};


pub struct KeyPair {
    expire: u64,
    timeout: u32,
    send_key: [u8; KEY],
    recv_key: [u8; KEY],
}

impl KeyPair {
    fn new(expire: u64, token: &PrivateToken) -> Self {
        Self {
            recv_key: *token.client_key(),
            send_key: *token.server_key(),
            timeout: token.timeout(),
            expire,
        }
    }

    pub fn send_key(&self) -> &[u8; KEY] { &self.send_key }
    pub fn recv_key(&self) -> &[u8; KEY] { &self.recv_key }
    pub fn timeout_secs(&self) -> u32 { self.timeout }
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(u64::from(self.timeout))
    }
}

pub struct Incoming {
    protocol: u64,
    timestamp: u64,
    private: [u8; KEY],
    key: [u8; KEY],
    sequence: AtomicU64,

    pending: HashMap<SocketAddr, KeyPair>,
    token_history: HashMap<[u8; HMAC], (SocketAddr, u64)>,
}

impl Incoming {
    pub fn new(protocol: u64, private: [u8; KEY]) -> Self {
        Self {
            protocol,
            private,
            key: keygen(),
            sequence: AtomicU64::new(0),
            timestamp: unix_time(),
            pending: HashMap::new(),
            token_history: HashMap::new(),
        }
    }

    pub fn open_request<'a>(&self, r: &'a mut Request) -> Result<(u64, &'a PrivateToken), ()> {
        if !r.is_valid(self.protocol, self.timestamp) { return Err(()) }
        r.open_token(&self.private)
    }

    pub fn open_response<'a>(&self, buf: &'a mut [u8; 8 + CHALLENGE_LEN], addr: &SocketAddr, seq: u64, prefix: u8, tag: &[u8; HMAC])
        -> Result<([u8; KEY], &'a ChallengeToken), ()>
    {
        let pending = self.pending.get(addr).ok_or(())?;
        Packet::open(self.protocol, buf, seq, prefix, tag, &pending.recv_key)?;
        let token = ChallengeToken::decode_packet(buf, &self.key)?;
        Ok((pending.send_key, token))
    }

    pub fn gen_challenge(&self, seq: u64, buf: &mut [u8], token: &PrivateToken) -> usize {
        let challenge_seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        let mut m = ChallengeToken::new(token.client_id(), *token.user())
            .encode_packet(challenge_seq, &self.key);
        Packet::encode_handshake(self.protocol, buf, seq, token.server_key(), &mut m).unwrap()
    }

    pub fn remove(&mut self, addr: &SocketAddr) -> Option<KeyPair> {
        self.pending.remove(addr)
    }
    pub fn insert(&mut self, addr: SocketAddr, expire: u64, token: &PrivateToken) {
        self.pending.entry(addr).or_insert_with(|| KeyPair::new(expire, &token));
    }
    pub fn add_token_history(&mut self, hmac: [u8; HMAC], addr: SocketAddr, expire: u64) -> bool {
        self.token_history.entry(hmac).or_insert((addr, expire)).0 == addr
    }
    pub fn update(&mut self) {
        let timestamp = unix_time();
        self.pending.retain(|_, p| p.expire > timestamp);
        self.token_history.retain(|_, v| v.1 > timestamp);
        self.timestamp = timestamp;
    }
}

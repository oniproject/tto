use std::{
    net::SocketAddr,
    time::{Instant, Duration},
    collections::hash_map::{HashMap, Entry},
};

use crate::crypto::Key;

pub struct Keys {
    timeout: Duration,
    last_access: Instant,
    send_key: Key,
    recv_key: Key,
}

impl Keys {
    pub fn expired(&self, time: Instant) -> bool {
        self.last_access + self.timeout < time
        //self.expire < time
    }
    pub fn disable_expire(&mut self) { unimplemented!() }
    pub fn send_key(&self) -> &Key { &self.send_key }
    pub fn recv_key(&self) -> &Key { &self.recv_key }
    pub fn timeout(&self) -> Duration { self.timeout }
}

pub struct Mapping {
    mapping: HashMap<SocketAddr, Keys>,
    time: Instant,
}

impl Mapping {
    pub fn new() -> Self {
        Self {
            mapping: HashMap::new(),
            time: Instant::now(),
        }
    }

    pub fn reset(&mut self) {
        //debug!("reset encryption manager");
        self.time = Instant::now();
        self.mapping.clear();
    }

    pub fn advance(&mut self) {
        self.time = Instant::now();
    }

    /*
    pub fn is_valid(&self, addr: SocketAddr) -> bool {
        self.mapping.contains_key(key.0)
    }

    pub fn is_expired(&self, key: EncryptionKey) -> bool {
        self.mapping.get(key.0).map(|e| e.expired(time)).unwrap_or(true)
    }
    */

    pub fn insert(&mut self, addr: SocketAddr, send_key: Key, recv_key: Key, timeout: u32) -> bool {
        self.mapping.insert(addr, Keys {
            send_key,
            recv_key,
            timeout: Duration::from_secs(timeout as u64),
            last_access: self.time,
        })
        .is_none()
    }

    pub fn remove(&mut self, addr: SocketAddr) -> bool {
        self.mapping.remove(&addr).is_some()
    }

    pub fn contains(&self, addr: SocketAddr) -> bool {
        self.mapping.contains_key(&addr)
    }

    pub fn find(&mut self, addr: SocketAddr) -> Option<&mut Keys> {
        match self.mapping.entry(addr) {
            Entry::Occupied(mut o) => {
                if !o.get().expired(self.time) {
                    o.get_mut().last_access = self.time;
                    Some(o.into_mut())
                } else {
                    o.remove_entry();
                    None
                }
            }
            Entry::Vacant(_) => None,
        }
    }

    pub fn touch(&mut self, addr: SocketAddr) -> bool {
        unimplemented!()
        /*
        match self.mapping.get(&addr) {
            Some(e) => { e.last_access = self.time; true }
            None => false,
        }
        */
    }

    /*
    pub fn expire(&self, addr: SocketAddr) -> Option<Instant> {
        self.mapping.get(key.0).map(|e| e.expire)
    }
    pub fn set_expire(&mut self, key: EncryptionKey, expire: Instant) -> bool {
        let e = self.mapping.get_mut(key.0);
        if let Some(e) = e { e.expire = expire }
        e.is_some()
    }
    pub fn send_key(&self, key: EncryptionKey) -> Option<&Key> {
        self.mapping.get(key.0).map(|e| &e.send_key)
    }
    */
    pub fn recv_key(&self, addr: SocketAddr) -> Option<&Key> {
        self.mapping.get(&addr).map(|e| &e.recv_key)
    }
}

#[test]
fn encryption_manager() {
    use crate::TEST_TIMEOUT_SECONDS;

    let mut manager = Mapping::new();

    // generate some test encryption mappings
    struct Map {
        id: usize,
        addr: SocketAddr,
        send_key: Key,
        recv_key: Key,
    }

    let mapping: Vec<_> = (0..5)
        .map(|id| Map {
            id: id,
            addr: format!("127.0.0.{}:{}", id + 1, 20000 + id).parse().unwrap(),
            send_key: Key::generate(),
            recv_key: Key::generate(),
        })
        .collect();

    let first = mapping.first().unwrap();
    let last = mapping.last().unwrap();

    // add the encryption mappings to the manager and make sure they can be looked up by addr
    for map in &mapping {
        assert!(manager.find(map.addr).is_none());
        assert!(manager.insert(
            map.addr,
            map.send_key.clone(),
            map.recv_key.clone(),
            TEST_TIMEOUT_SECONDS,
        ));
        let e = manager.find(map.addr).unwrap();
        assert_eq!(e.send_key(), &map.send_key);
        assert_eq!(e.recv_key(), &map.recv_key);
    }

    // removing an encryption mapping that doesn't exist should return 0
    {
        let addr = format!("127.0.0.{}:{}", 1, 50000).parse().unwrap();
        assert!(!manager.remove(addr));
    }

    // remove the first and last encryption mappings
    assert!(manager.remove(first.addr));
    assert!(manager.remove(last.addr));

    // make sure the encryption mappings that were removed can no longer be looked up by addr
    for map in &mapping {
        let e = manager.find(map.addr);
        if map.addr == first.addr || map.addr == last.addr {
            assert!(e.is_none());
        } else {
            let e = e.unwrap();
            assert_eq!(e.send_key(), &map.send_key);
            assert_eq!(e.recv_key(), &map.recv_key);
        }
    }

    // add the encryption mappings back in
    assert!(manager.insert(
        first.addr,
        first.send_key.clone(),
        first.recv_key.clone(),
        TEST_TIMEOUT_SECONDS,
    ));

    assert!(manager.insert(
        last.addr,
        last.send_key.clone(),
        last.recv_key.clone(),
        TEST_TIMEOUT_SECONDS,
    ));

    // all encryption mappings should be able to be looked up by addr again
    for map in &mapping {
        let e = manager.find(map.addr).unwrap();
        assert_eq!(e.send_key(), &map.send_key);
        assert_eq!(e.recv_key(), &map.recv_key);
    }

    // check that encryption mappings time out properly
    manager.time += Duration::from_secs(2 * TEST_TIMEOUT_SECONDS as u64);

    for map in &mapping {
        assert!(manager.find(map.addr).is_none());
    }

    // add the same encryption mappings after timeout
    for map in &mapping {
        assert!(manager.find(map.addr).is_none());
        assert!(manager.insert(
            map.addr,
            map.send_key.clone(),
            map.recv_key.clone(),
            TEST_TIMEOUT_SECONDS,
        ));
        let e = manager.find(map.addr).unwrap();
        assert_eq!(e.send_key(), &map.send_key);
        assert_eq!(e.recv_key(), &map.recv_key);
    }

    // reset the encryption mapping and verify that all encryption mappings have been removed
    manager.reset();

    for map in &mapping {
        assert!(manager.find(map.addr).is_none());
    }

    // test the expire time for encryption mapping works as expected
    assert!(manager.insert(
        first.addr,
        first.send_key.clone(),
        first.recv_key.clone(),
        TEST_TIMEOUT_SECONDS,
    ));

    /*
    let idx = manager.find_mapping(first.addr, time);
    assert!(!idx.is_null());
    assert!(manager.find_mapping(first.addr, time + 1.1).is_null());
    //manager.set_expire_time(idx, -1.0);
    assert!(manager.find(first.addr).is_some());
    */
}

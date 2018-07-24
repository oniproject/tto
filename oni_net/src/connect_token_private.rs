use crypto::{encrypt_aead, decrypt_aead, MAC_BYTES, Key, ReadKey, WriteKey};
use addr::{ReadIps, WriteIps, MAX_SERVERS_PER_CONNECT};
use utils::{UserData, ReadUserData, WriteUserData};
use VERSION_INFO_BYTES;

use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use std::net::SocketAddr;
use std::io::{self, Write};

pub const CONNECT_TOKEN_PRIVATE_BYTES: usize = 1024;

struct ConnectTokenPrivate {
    client_id: u64,
    timeout_seconds: u32,
    server_addresses: Vec<SocketAddr>,
    client_to_server_key: Key,
    server_to_client_key: Key,
    user_data: UserData,
}

impl ConnectTokenPrivate {
    pub fn generate(client_id: u64, timeout_seconds: u32, addresses: Vec<SocketAddr>, user_data: UserData) -> Self {
        assert!(addresses.len() > 0);
        assert!(addresses.len() <= MAX_SERVERS_PER_CONNECT);
        Self {
            client_id,
            timeout_seconds,
            server_addresses: addresses,
            client_to_server_key: Key::generate(),
            server_to_client_key: Key::generate(),
            user_data,
        }
    }

    pub fn read(mut buffer: &[u8]) -> io::Result<Self> {
        Ok(Self {
            client_id: buffer.read_u64::<LE>()?,
            timeout_seconds: buffer.read_u32::<LE>()?,
            server_addresses: buffer.read_ips()?,
            client_to_server_key: buffer.read_key()?,
            server_to_client_key: buffer.read_key()?,
            user_data: buffer.read_user_data()?,
        })
    }


    pub fn write(&self, mut buffer: &mut [u8]) -> io::Result<()> {
        buffer.write_u64::<LE>(self.client_id)?;
        buffer.write_u32::<LE>(self.timeout_seconds)?;
        buffer.write_ips(&self.server_addresses)?;
        buffer.write_key(&self.client_to_server_key)?;
        buffer.write_key(&self.server_to_client_key)?;
        buffer.write_user_data(&self.user_data)
    }

    pub fn encrypt(
        buffer: &mut [u8],
        version_info: &[u8],
        protocol_id: u64,
        expire_timestamp: u64,
        sequence: u64,
        key: &Key) -> io::Result<()>
    {
        assert!(buffer.len() == CONNECT_TOKEN_PRIVATE_BYTES);

        let mut additional = [0u8; VERSION_INFO_BYTES + 8 + 8];
        {
            let mut p = &mut additional[..];
            p.write_all(&::VERSION_INFO[..]).unwrap();
            p.write_u64::<LE>(protocol_id).unwrap();
            p.write_u64::<LE>(expire_timestamp).unwrap();
        }

        let nonce = Nonce::from_sequence(sequence);
        let len = CONNECT_TOKEN_PRIVATE_BYTES - MAC_BYTES;
        encrypt_aead(&mut buffer[..len], &additional[..], &nonce, key)
    }

    pub fn decrypt(
        buffer: &mut [u8],
        version_info: &[u8],
        protocol_id: u64,
        expire_timestamp: u64,
        sequence: u64,
        key: &Key) -> io::Result<()>
    {
        assert!(buffer.len() == CONNECT_TOKEN_PRIVATE_BYTES);

        let mut additional = [0u8; VERSION_INFO_BYTES + 8 + 8];
        {
            let mut p = &mut additional[..];
            p.write_all(&::VERSION_INFO[..]).unwrap();
            p.write_u64::<LE>(protocol_id).unwrap();
            p.write_u64::<LE>(expire_timestamp).unwrap();
        }

        let nonce = Nonce::from_sequence(sequence);
        let len = CONNECT_TOKEN_PRIVATE_BYTES;
        decrypt_aead(&mut buffer[..len], &additional[..], &nonce, key)
    }
}

const TEST_PROTOCOL_ID: u64 = 0x1122334455667788;
const TEST_CLIENT_ID: u64 = 0x1;
//const TEST_SERVER_PORT:             40000,
//const TEST_CONNECT_TOKEN_EXPIRY   30,
const TEST_TIMEOUT_SECONDS: u32 = 15;

#[test]
fn connect_token() {
    use std::time::SystemTime;

    // generate a connect token
    let server_address = "127.0.0.1:40000".parse().unwrap();

    let mut user_data = [0u8; ::utils::USER_DATA_BYTES];
    ::crypto::random_bytes(&mut user_data[..]);
    let user_data: UserData = user_data.into();

    let input_token = ConnectTokenPrivate::generate(TEST_CLIENT_ID, TEST_TIMEOUT_SECONDS, vec![server_address], user_data.clone());

    assert_eq!(input_token.client_id, TEST_CLIENT_ID);
    assert_eq!(input_token.server_addresses, &[server_address]);
    assert_eq!(input_token.user_data, user_data);

    // write it to a buffer

    let mut buffer = [0u8; CONNECT_TOKEN_PRIVATE_BYTES];
    input_token.write(&mut buffer[..]).unwrap();

    // encrypt/decrypt the buffer

    let sequence = 1000u64;
    let expire_timestamp: u64 = 30 + SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
    let key = Key::generate();

    ConnectTokenPrivate::encrypt(
        &mut buffer[..],
        ::VERSION_INFO,
        TEST_PROTOCOL_ID,
        expire_timestamp,
        sequence,
        &key).unwrap();

    ConnectTokenPrivate::decrypt(
        &mut buffer[..],
        ::VERSION_INFO,
        TEST_PROTOCOL_ID,
        expire_timestamp,
        sequence,
        &key).unwrap();

    // read the connect token back in

    let output_token = ConnectTokenPrivate::read(&mut buffer[..]).unwrap();

    // make sure that everything matches the original connect token

    assert_eq!(output_token.client_id, input_token.client_id);
    assert_eq!(output_token.timeout_seconds, input_token.timeout_seconds);
    assert_eq!(output_token.client_to_server_key, input_token.client_to_server_key);
    assert_eq!(output_token.server_to_client_key, input_token.server_to_client_key);
    assert_eq!(output_token.user_data, input_token.user_data);
    assert_eq!(&output_token.server_addresses[..], &input_token.server_addresses[..]);
}

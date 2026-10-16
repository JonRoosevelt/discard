use std::collections::{HashMap, HashSet};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use dashmap::mapref::entry::Entry;
use dashmap::{DashMap, DashSet};
use hmac::{Hmac, KeyInit, Mac};
use iroh::endpoint::Connection;
use iroh::protocol::{AcceptError, ProtocolHandler};
use iroh::{Endpoint, EndpointId};
use md5::{Digest, Md5};
use sha1::Sha1;
use shared::IceServer;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Semaphore};

use crate::iroh_net::OP_TIMEOUT;

pub const ALPN: &[u8] = b"discard/media/1";

const REALM: &str = "discard";
const MAGIC: u32 = 0x2112_A442;
const LIFETIME: u32 = 600;
const OUT_QUEUE: usize = 512;
const MAX_PAYLOAD: usize = 1500;
const MAX_ALLOCS: usize = 256;
const MAX_CLIENTS: usize = 64;
const MAX_AUTH_FAILURES: u8 = 5;
const UNAUTH_TIMEOUT: Duration = Duration::from_secs(10);
const FRAGMENT_TTL: Duration = Duration::from_secs(1);
const PORT_BASE: u16 = 49152;
const PORT_SPAN: u16 = 16384;

const ALLOCATE: u16 = 0x0003;
const REFRESH: u16 = 0x0004;
const SEND: u16 = 0x0016;
const DATA: u16 = 0x0017;
const CREATE_PERMISSION: u16 = 0x0008;
const CHANNEL_BIND: u16 = 0x0009;

const A_USERNAME: u16 = 0x0006;
const A_INTEGRITY: u16 = 0x0008;
const A_ERROR: u16 = 0x0009;
const A_CHANNEL: u16 = 0x000C;
const A_LIFETIME: u16 = 0x000D;
const A_PEER: u16 = 0x0012;
const A_DATA: u16 = 0x0013;
const A_REALM: u16 = 0x0014;
const A_NONCE: u16 = 0x0015;
const A_RELAYED: u16 = 0x0016;
const A_TRANSPORT: u16 = 0x0019;
const A_MAPPED: u16 = 0x0020;

// Relayed addresses are virtual: they only ever resolve to a known peer node over iroh, so the
// TURN server can never be used to reach a real host. 100.64.0.0/10 keeps them out of LAN ranges.
pub fn virtual_ip(id: &EndpointId) -> Ipv4Addr {
    let h = blake3::hash(id.as_bytes());
    let n = u32::from_be_bytes([
        h.as_bytes()[0],
        h.as_bytes()[1],
        h.as_bytes()[2],
        h.as_bytes()[3],
    ]);
    Ipv4Addr::from(0x6440_0000 | (n & 0x003F_FFFF))
}

pub struct Turn {
    port: u16,
    username: String,
    password: String,
    key: [u8; 16],
    nonce: String,
    endpoint: RwLock<Option<Endpoint>>,
    peers: DashMap<Ipv4Addr, EndpointId>,
    conns: DashMap<EndpointId, Connection>,
    connecting: DashSet<EndpointId>,
    slots: Arc<Semaphore>,
    allocs: DashMap<u16, Arc<Alloc>>,
    next_port: AtomicU16,
    next_seq: AtomicU16,
}

impl std::fmt::Debug for Turn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Turn").field("port", &self.port).finish()
    }
}

struct Alloc {
    perms: Mutex<HashSet<Ipv4Addr>>,
    channels: Mutex<HashMap<u16, SocketAddrV4>>,
    out: mpsc::Sender<Vec<u8>>,
}

impl Turn {
    pub async fn start() -> anyhow::Result<Arc<Self>> {
        // TCP, not UDP: Chrome binds its TURN sockets to each interface address, and macOS
        // refuses to send from a 127.0.0.1-bound UDP socket to a non-loopback source, so UDP
        // replies never arrive. A 127.0.0.1 TCP listener accepts those connections fine.
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let username = hex::encode(rand::random::<[u8; 8]>());
        let password = hex::encode(rand::random::<[u8; 16]>());
        let key: [u8; 16] = Md5::digest(format!("{username}:{REALM}:{password}")).into();
        let turn = Arc::new(Turn {
            port: listener.local_addr()?.port(),
            username,
            password,
            key,
            nonce: hex::encode(rand::random::<[u8; 8]>()),
            endpoint: RwLock::new(None),
            peers: DashMap::new(),
            conns: DashMap::new(),
            connecting: DashSet::new(),
            slots: Arc::new(Semaphore::new(MAX_CLIENTS)),
            allocs: DashMap::new(),
            next_port: AtomicU16::new(0),
            next_seq: AtomicU16::new(0),
        });
        tracing::info!("turn listening on 127.0.0.1:{}", turn.port);
        let accept = turn.clone();
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        let Ok(slot) = accept.slots.clone().try_acquire_owned() else {
                            continue;
                        };
                        let turn = accept.clone();
                        tokio::spawn(async move {
                            turn.serve_client(stream, addr).await;
                            drop(slot);
                        });
                    }
                    Err(e) => {
                        tracing::warn!("turn accept failed: {e}");
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
            }
        });
        Ok(turn)
    }

    pub fn ice_server(&self) -> IceServer {
        IceServer {
            urls: vec![format!("turn:127.0.0.1:{}?transport=tcp", self.port)],
            username: self.username.clone(),
            credential: self.password.clone(),
        }
    }

    pub fn set_endpoint(&self, endpoint: Endpoint) {
        *self.endpoint.write().unwrap() = Some(endpoint);
        self.conns.clear();
    }

    pub fn add_peer(&self, id: &str) {
        let Ok(id) = id.parse::<EndpointId>() else {
            return;
        };
        let ip = virtual_ip(&id);
        let existing = *self.peers.entry(ip).or_insert(id);
        if existing != id {
            tracing::warn!(
                "virtual address {ip} collides for {existing} and {id}; keeping the first"
            );
        }
    }

    fn known_peer(&self, ip: Ipv4Addr) -> Option<EndpointId> {
        self.peers.get(&ip).map(|p| *p)
    }

    fn endpoint(&self) -> Option<Endpoint> {
        self.endpoint.read().unwrap().clone()
    }

    async fn serve_client(self: Arc<Self>, stream: TcpStream, addr: SocketAddr) {
        let _ = stream.set_nodelay(true);
        let (mut reader, mut writer) = stream.into_split();
        let (out, mut rx) = mpsc::channel::<Vec<u8>>(OUT_QUEUE);
        tokio::spawn(async move {
            while let Some(buf) = rx.recv().await {
                if writer.write_all(&buf).await.is_err() {
                    break;
                }
            }
        });
        let mut client = Client {
            turn: self.clone(),
            addr,
            out,
            alloc: None,
            expires: Instant::now(),
            auth_failures: 0,
        };
        let mut buf = vec![0u8; 4];
        loop {
            let deadline = match client.alloc {
                Some(_) => client.expires,
                None => Instant::now() + UNAUTH_TIMEOUT,
            };
            let read =
                tokio::time::timeout_at(deadline.into(), read_message(&mut reader, &mut buf));
            let Ok(Some(len)) = read.await else {
                break;
            };
            if buf[0] >= 0x40 {
                client.channel_data(&buf[..4 + len]);
            } else if let Some(msg) = Msg::parse(&buf) {
                if !client.handle(&buf, msg).await {
                    break;
                }
            }
        }
        if let Some((port, _)) = client.alloc {
            self.allocs.remove(&port);
        }
    }

    fn allocate_port(&self, alloc: &Arc<Alloc>) -> Option<u16> {
        if self.allocs.len() >= MAX_ALLOCS {
            return None;
        }
        for _ in 0..PORT_SPAN {
            let port = PORT_BASE + self.next_port.fetch_add(1, Ordering::Relaxed) % PORT_SPAN;
            if let Entry::Vacant(slot) = self.allocs.entry(port) {
                slot.insert(alloc.clone());
                return Some(port);
            }
        }
        None
    }

    fn forward(self: &Arc<Self>, peer: SocketAddrV4, src_port: u16, payload: &[u8]) {
        if payload.len() > MAX_PAYLOAD {
            return;
        }
        let Some(id) = self.known_peer(*peer.ip()) else {
            return;
        };
        let conn = self
            .conns
            .get(&id)
            .filter(|c| c.close_reason().is_none())
            .map(|c| c.clone());
        let Some(conn) = conn else {
            self.connect(id);
            return;
        };
        let max = conn.max_datagram_size().unwrap_or(0);
        let seq = self.next_seq.fetch_add(1, Ordering::Relaxed);
        for frame in frames(peer.port(), src_port, seq, payload, max) {
            if let Err(e) = conn.send_datagram(frame.into()) {
                tracing::debug!(
                    "media datagram dropped (payload {}, max {max}): {e}",
                    payload.len()
                );
            }
        }
    }

    fn connect(self: &Arc<Self>, id: EndpointId) {
        if !self.connecting.insert(id) {
            return;
        }
        let Some(endpoint) = self.endpoint() else {
            self.connecting.remove(&id);
            return;
        };
        let turn = self.clone();
        tokio::spawn(async move {
            let res = tokio::time::timeout(OP_TIMEOUT, endpoint.connect(id, ALPN)).await;
            turn.connecting.remove(&id);
            match res {
                Ok(Ok(conn)) => turn.pump(conn).await,
                Ok(Err(e)) => tracing::debug!("media connect to {id} failed: {e}"),
                Err(_) => tracing::debug!("media connect to {id} timed out"),
            }
        });
    }

    async fn pump(&self, conn: Connection) {
        let remote = conn.remote_id();
        let src_ip = virtual_ip(&remote);
        let relay = conn.paths().iter().any(|p| p.is_selected() && p.is_relay());
        tracing::info!(
            "media link to {remote} up (relay path: {relay}, max datagram {:?})",
            conn.max_datagram_size()
        );
        self.conns.insert(remote, conn.clone());
        let mut reassembly = Reassembly::default();
        while let Ok(d) = conn.read_datagram().await {
            if self.known_peer(src_ip) != Some(remote) {
                continue;
            }
            let Some((dst_port, src_port, payload)) = reassembly.push(&d) else {
                continue;
            };
            if let Some(alloc) = self.allocs.get(&dst_port).map(|a| a.clone()) {
                alloc.deliver(SocketAddrV4::new(src_ip, src_port), &payload);
            }
        }
        self.conns
            .remove_if(&remote, |_, c| c.stable_id() == conn.stable_id());
    }
}

impl ProtocolHandler for Turn {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let remote = connection.remote_id();
        if self.known_peer(virtual_ip(&remote)) != Some(remote) {
            connection.close(0u32.into(), b"unknown peer");
            return Ok(());
        }
        self.pump(connection).await;
        Ok(())
    }
}

impl Alloc {
    fn deliver(&self, src: SocketAddrV4, payload: &[u8]) {
        if !self.perms.lock().unwrap().contains(src.ip()) {
            return;
        }
        let channel = self
            .channels
            .lock()
            .unwrap()
            .iter()
            .find(|(_, p)| **p == src)
            .map(|(c, _)| *c);
        let buf = match channel {
            Some(c) => channel_data(c, payload),
            None => encode(
                DATA,
                &rand::random::<[u8; 12]>(),
                &[(A_PEER, xor_addr(src)), (A_DATA, payload.to_vec())],
                None,
            ),
        };
        let _ = self.out.try_send(buf);
    }
}

type Reply = Result<Vec<(u16, Vec<u8>)>, (u16, &'static str)>;

struct Client {
    turn: Arc<Turn>,
    addr: SocketAddr,
    out: mpsc::Sender<Vec<u8>>,
    alloc: Option<(u16, Arc<Alloc>)>,
    expires: Instant,
    auth_failures: u8,
}

impl Client {
    fn channel_data(&self, buf: &[u8]) {
        let Some((port, alloc)) = &self.alloc else {
            return;
        };
        let number = u16::from_be_bytes([buf[0], buf[1]]);
        let peer = alloc.channels.lock().unwrap().get(&number).copied();
        if let Some(peer) = peer {
            self.turn.forward(peer, *port, &buf[4..]);
        }
    }

    async fn handle(&mut self, raw: &[u8], msg: Msg) -> bool {
        if msg.typ == SEND {
            let (Some((port, alloc)), Some(peer), Some(data)) = (
                &self.alloc,
                msg.attr(A_PEER).and_then(parse_xor_addr),
                msg.attr(A_DATA),
            ) else {
                return true;
            };
            if alloc.perms.lock().unwrap().contains(peer.ip()) {
                self.turn.forward(peer, *port, data);
            }
            return true;
        }
        if !matches!(
            msg.typ,
            ALLOCATE | REFRESH | CREATE_PERMISSION | CHANNEL_BIND
        ) {
            return true;
        }
        if !self.authenticated(raw, &msg) {
            self.auth_failures += 1;
            let attrs = [
                (A_ERROR, error_code(401, "Unauthorized")),
                (A_REALM, REALM.as_bytes().to_vec()),
                (A_NONCE, self.turn.nonce.as_bytes().to_vec()),
            ];
            self.reply(encode(msg.typ | 0x0110, &msg.tid, &attrs, None))
                .await;
            return self.auth_failures < MAX_AUTH_FAILURES;
        }
        self.auth_failures = 0;
        let result = match msg.typ {
            ALLOCATE => self.allocate(&msg),
            REFRESH => self.refresh(&msg),
            CREATE_PERMISSION => self.create_permission(&msg),
            _ => self.channel_bind(&msg),
        };
        let buf = match result {
            Ok(attrs) => encode(msg.typ | 0x0100, &msg.tid, &attrs, Some(&self.turn.key)),
            Err((code, reason)) => encode(
                msg.typ | 0x0110,
                &msg.tid,
                &[(A_ERROR, error_code(code, reason))],
                Some(&self.turn.key),
            ),
        };
        self.reply(buf).await;
        true
    }

    async fn reply(&self, buf: Vec<u8>) {
        let _ = self.out.send(buf).await;
    }

    fn authenticated(&self, raw: &[u8], msg: &Msg) -> bool {
        msg.attr(A_USERNAME) == Some(self.turn.username.as_bytes())
            && msg.attr(A_REALM) == Some(REALM.as_bytes())
            && msg.attr(A_NONCE) == Some(self.turn.nonce.as_bytes())
            && msg.integrity_ok(raw, &self.turn.key)
    }

    fn allocate(&mut self, msg: &Msg) -> Reply {
        if self.alloc.is_some() {
            return Err((437, "Allocation Mismatch"));
        }
        match msg.attr(A_TRANSPORT).and_then(|v| v.first()) {
            None => return Err((400, "Bad Request")),
            Some(17) => {}
            Some(_) => return Err((442, "Unsupported Transport Protocol")),
        }
        let endpoint = self.turn.endpoint().ok_or((508, "Insufficient Capacity"))?;
        let alloc = Arc::new(Alloc {
            perms: Mutex::new(HashSet::new()),
            channels: Mutex::new(HashMap::new()),
            out: self.out.clone(),
        });
        let port = self
            .turn
            .allocate_port(&alloc)
            .ok_or((508, "Insufficient Capacity"))?;
        self.alloc = Some((port, alloc));
        self.expires = Instant::now() + Duration::from_secs(LIFETIME.into());
        let relayed = SocketAddrV4::new(virtual_ip(&endpoint.id()), port);
        let mapped = match self.addr {
            SocketAddr::V4(a) => a,
            SocketAddr::V6(_) => SocketAddrV4::new(Ipv4Addr::LOCALHOST, self.addr.port()),
        };
        Ok(vec![
            (A_RELAYED, xor_addr(relayed)),
            (A_MAPPED, xor_addr(mapped)),
            (A_LIFETIME, LIFETIME.to_be_bytes().to_vec()),
        ])
    }

    fn refresh(&mut self, msg: &Msg) -> Reply {
        let Some((port, _)) = &self.alloc else {
            return Err((437, "Allocation Mismatch"));
        };
        let lifetime = match msg.attr(A_LIFETIME) {
            Some(&[a, b, c, d]) if u32::from_be_bytes([a, b, c, d]) == 0 => 0,
            _ => LIFETIME,
        };
        if lifetime == 0 {
            self.turn.allocs.remove(port);
            self.alloc = None;
        } else {
            self.expires = Instant::now() + Duration::from_secs(lifetime.into());
        }
        Ok(vec![(A_LIFETIME, lifetime.to_be_bytes().to_vec())])
    }

    fn permitted_peer(&self, value: &[u8]) -> Result<SocketAddrV4, (u16, &'static str)> {
        parse_xor_addr(value)
            .filter(|p| self.turn.known_peer(*p.ip()).is_some())
            .ok_or((403, "Forbidden"))
    }

    fn create_permission(&mut self, msg: &Msg) -> Reply {
        let Some((_, alloc)) = &self.alloc else {
            return Err((437, "Allocation Mismatch"));
        };
        let peers = msg
            .attrs
            .iter()
            .filter(|(t, _)| *t == A_PEER)
            .map(|(_, v)| self.permitted_peer(v))
            .collect::<Result<Vec<_>, _>>()?;
        if peers.is_empty() {
            return Err((400, "Bad Request"));
        }
        alloc
            .perms
            .lock()
            .unwrap()
            .extend(peers.iter().map(|p| *p.ip()));
        Ok(vec![])
    }

    fn channel_bind(&mut self, msg: &Msg) -> Reply {
        let Some((_, alloc)) = &self.alloc else {
            return Err((437, "Allocation Mismatch"));
        };
        let number = match msg.attr(A_CHANNEL) {
            Some(&[a, b, ..]) => u16::from_be_bytes([a, b]),
            _ => return Err((400, "Bad Request")),
        };
        let peer = self.permitted_peer(msg.attr(A_PEER).ok_or((400, "Bad Request"))?)?;
        if !(0x4000..=0x4FFF).contains(&number) {
            return Err((400, "Bad Request"));
        }
        let mut channels = alloc.channels.lock().unwrap();
        let conflict = channels.iter().any(|(c, p)| (*c == number) != (*p == peer));
        if conflict {
            return Err((400, "Bad Request"));
        }
        channels.insert(number, peer);
        alloc.perms.lock().unwrap().insert(*peer.ip());
        Ok(vec![])
    }
}

const WHOLE: u8 = 0;
const HALF: u8 = 1;

// Chrome sends ~1200-byte RTP packets but a QUIC path starts at ~1160 bytes of datagram room
// and stays there if path MTU discovery fails (VPNs, relay paths), so oversized packets go in
// two halves instead of being dropped.
fn frames(dst_port: u16, src_port: u16, seq: u16, payload: &[u8], max: usize) -> Vec<Vec<u8>> {
    let head = |kind: u8| {
        let mut f = vec![kind];
        f.extend_from_slice(&dst_port.to_be_bytes());
        f.extend_from_slice(&src_port.to_be_bytes());
        f
    };
    if payload.len() + 5 <= max {
        let mut f = head(WHOLE);
        f.extend_from_slice(payload);
        return vec![f];
    }
    let (a, b) = payload.split_at(payload.len() / 2);
    [a, b]
        .iter()
        .enumerate()
        .map(|(part, chunk)| {
            let mut f = head(HALF);
            f.extend_from_slice(&seq.to_be_bytes());
            f.push(part as u8);
            f.extend_from_slice(chunk);
            f
        })
        .collect()
}

type Fragment = (u8, Vec<u8>, Instant);

#[derive(Default)]
struct Reassembly {
    pending: HashMap<(u16, u16, u16), Fragment>,
}

impl Reassembly {
    fn push(&mut self, d: &[u8]) -> Option<(u16, u16, Vec<u8>)> {
        self.push_at(d, Instant::now())
    }

    fn push_at(&mut self, d: &[u8], now: Instant) -> Option<(u16, u16, Vec<u8>)> {
        self.pending
            .retain(|_, (_, _, t)| now.duration_since(*t) < FRAGMENT_TTL);
        let dst = u16::from_be_bytes([*d.get(1)?, *d.get(2)?]);
        let src = u16::from_be_bytes([*d.get(3)?, *d.get(4)?]);
        match d[0] {
            WHOLE if d.len() - 5 <= MAX_PAYLOAD => Some((dst, src, d[5..].to_vec())),
            HALF if (8..=8 + MAX_PAYLOAD).contains(&d.len()) => {
                let key = (dst, src, u16::from_be_bytes([d[5], d[6]]));
                let part = d[7];
                match self.pending.remove(&key) {
                    Some((other, mut first, _)) if other != part => {
                        if part == 0 {
                            first.splice(0..0, d[8..].iter().copied());
                        } else {
                            first.extend_from_slice(&d[8..]);
                        }
                        (first.len() <= MAX_PAYLOAD).then_some((dst, src, first))
                    }
                    _ => {
                        if self.pending.len() >= 64 {
                            self.pending.clear();
                        }
                        self.pending.insert(key, (part, d[8..].to_vec(), now));
                        None
                    }
                }
            }
            _ => None,
        }
    }
}

#[derive(Debug)]
struct Msg {
    typ: u16,
    tid: [u8; 12],
    attrs: Vec<(u16, Vec<u8>)>,
    integrity_at: Option<usize>,
}

impl Msg {
    fn parse(buf: &[u8]) -> Option<Msg> {
        if buf.len() < 20 || u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]) != MAGIC {
            return None;
        }
        let typ = u16::from_be_bytes([buf[0], buf[1]]);
        let len = u16::from_be_bytes([buf[2], buf[3]]) as usize;
        let body = buf.get(20..20 + len)?;
        let mut tid = [0u8; 12];
        tid.copy_from_slice(&buf[8..20]);
        let mut attrs = Vec::new();
        let mut integrity_at = None;
        let mut i = 0;
        while i + 4 <= body.len() {
            let t = u16::from_be_bytes([body[i], body[i + 1]]);
            let l = u16::from_be_bytes([body[i + 2], body[i + 3]]) as usize;
            let v = body.get(i + 4..i + 4 + l)?;
            if t == A_INTEGRITY {
                integrity_at = Some(20 + i);
            }
            if integrity_at.is_none() || t == A_INTEGRITY {
                attrs.push((t, v.to_vec()));
            }
            i += 4 + l.div_ceil(4) * 4;
        }
        Some(Msg {
            typ,
            tid,
            attrs,
            integrity_at,
        })
    }

    fn attr(&self, t: u16) -> Option<&[u8]> {
        self.attrs
            .iter()
            .find(|(k, _)| *k == t)
            .map(|(_, v)| v.as_slice())
    }

    fn integrity_ok(&self, raw: &[u8], key: &[u8]) -> bool {
        let (Some(at), Some(expected)) = (self.integrity_at, self.attr(A_INTEGRITY)) else {
            return false;
        };
        let mut head = raw[..at].to_vec();
        head[2..4].copy_from_slice(&((at - 20 + 24) as u16).to_be_bytes());
        let mut mac =
            <Hmac<Sha1> as KeyInit>::new_from_slice(key).expect("hmac accepts any key size");
        mac.update(&head);
        mac.verify_slice(expected).is_ok()
    }
}

async fn read_message(reader: &mut OwnedReadHalf, buf: &mut Vec<u8>) -> Option<usize> {
    buf.resize(4, 0);
    reader.read_exact(&mut buf[..4]).await.ok()?;
    let len = u16::from_be_bytes([buf[2], buf[3]]) as usize;
    let total = match buf[0] {
        0x00..=0x3F => 20 + len,
        0x40..=0x4F => 4 + len.div_ceil(4) * 4,
        _ => return None,
    };
    buf.resize(total, 0);
    reader.read_exact(&mut buf[4..]).await.ok()?;
    Some(len)
}

fn hmac_sha1(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = <Hmac<Sha1> as KeyInit>::new_from_slice(key).expect("hmac accepts any key size");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn encode(typ: u16, tid: &[u8; 12], attrs: &[(u16, Vec<u8>)], key: Option<&[u8; 16]>) -> Vec<u8> {
    let mut buf = Vec::with_capacity(64);
    buf.extend_from_slice(&typ.to_be_bytes());
    buf.extend_from_slice(&[0, 0]);
    buf.extend_from_slice(&MAGIC.to_be_bytes());
    buf.extend_from_slice(tid);
    for (t, v) in attrs {
        put_attr(&mut buf, *t, v);
    }
    if let Some(key) = key {
        let len = (buf.len() - 20 + 24) as u16;
        buf[2..4].copy_from_slice(&len.to_be_bytes());
        let mac = hmac_sha1(key, &buf);
        put_attr(&mut buf, A_INTEGRITY, &mac);
    }
    let len = (buf.len() - 20) as u16;
    buf[2..4].copy_from_slice(&len.to_be_bytes());
    buf
}

fn put_attr(buf: &mut Vec<u8>, t: u16, v: &[u8]) {
    buf.extend_from_slice(&t.to_be_bytes());
    buf.extend_from_slice(&(v.len() as u16).to_be_bytes());
    buf.extend_from_slice(v);
    buf.resize(buf.len().div_ceil(4) * 4, 0);
}

fn channel_data(number: u16, payload: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(4 + payload.len() + 3);
    buf.extend_from_slice(&number.to_be_bytes());
    buf.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    buf.extend_from_slice(payload);
    buf.resize(buf.len().div_ceil(4) * 4, 0);
    buf
}

fn error_code(code: u16, reason: &str) -> Vec<u8> {
    let mut v = vec![0, 0, (code / 100) as u8, (code % 100) as u8];
    v.extend_from_slice(reason.as_bytes());
    v
}

fn xor_addr(addr: SocketAddrV4) -> Vec<u8> {
    let port = addr.port() ^ (MAGIC >> 16) as u16;
    let ip = u32::from(*addr.ip()) ^ MAGIC;
    let mut v = vec![0, 1];
    v.extend_from_slice(&port.to_be_bytes());
    v.extend_from_slice(&ip.to_be_bytes());
    v
}

fn parse_xor_addr(v: &[u8]) -> Option<SocketAddrV4> {
    if v.len() != 8 || v[1] != 1 {
        return None;
    }
    let port = u16::from_be_bytes([v[2], v[3]]) ^ (MAGIC >> 16) as u16;
    let ip = u32::from_be_bytes([v[4], v[5], v[6], v[7]]) ^ MAGIC;
    Some(SocketAddrV4::new(Ipv4Addr::from(ip), port))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stun_roundtrip_and_media_frames() {
        let key = [7u8; 16];
        let tid = [9u8; 12];
        let peer = SocketAddrV4::new(Ipv4Addr::new(100, 64, 1, 2), 50000);
        let raw = encode(
            CREATE_PERMISSION,
            &tid,
            &[(A_USERNAME, b"user".to_vec()), (A_PEER, xor_addr(peer))],
            Some(&key),
        );
        let msg = Msg::parse(&raw).unwrap();
        assert_eq!(msg.typ, CREATE_PERMISSION);
        assert_eq!(msg.attr(A_USERNAME), Some(&b"user"[..]));
        assert_eq!(msg.attr(A_PEER).and_then(parse_xor_addr), Some(peer));
        assert!(msg.integrity_ok(&raw, &key));
        assert!(!msg.integrity_ok(&raw, &[8u8; 16]));
        let mut tampered = raw.clone();
        tampered[27] ^= 1;
        assert!(!Msg::parse(&tampered).unwrap().integrity_ok(&tampered, &key));

        let payload: Vec<u8> = (0..1200u32).map(|i| i as u8).collect();
        let mut r = Reassembly::default();
        let whole = frames(1, 2, 0, &payload, 1414);
        assert_eq!(whole.len(), 1);
        assert_eq!(r.push(&whole[0]), Some((1, 2, payload.clone())));
        let halves = frames(1, 2, 5, &payload, 1162);
        assert_eq!(halves.len(), 2);
        assert!(halves.iter().all(|f| f.len() <= 1162));
        assert_eq!(r.push(&halves[1]), None);
        assert_eq!(r.push(&halves[0]), Some((1, 2, payload.clone())));

        let big = vec![0u8; MAX_PAYLOAD + 1];
        assert_eq!(r.push(&frames(1, 2, 6, &big, 2000)[0]), None);
        let oversize = frames(1, 2, 7, &vec![0u8; 2 * MAX_PAYLOAD], 1162);
        assert_eq!(r.push(&oversize[0]), None);
        assert_eq!(r.push(&oversize[1]), None);
        assert!(r.pending.is_empty());

        let now = Instant::now();
        let halves = frames(1, 2, 9, &payload, 1162);
        assert_eq!(r.push_at(&halves[0], now), None);
        assert_eq!(r.push_at(&halves[1], now + FRAGMENT_TTL * 2), None);
        assert_eq!(r.pending.len(), 1);
    }
}

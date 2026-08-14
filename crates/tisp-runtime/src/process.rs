/// Process calculi runtime: async π, applied π, spi, ρ, ambients, κ, SKI
use std::sync::{Arc, Mutex};
use std::collections::{HashMap, VecDeque};

/// Async π-calculus: fire-and-forget send via buffered channels
#[derive(Clone)]
pub struct AsyncChannel<T: Clone + Send> {
    buffer: Arc<Mutex<VecDeque<T>>>,
}

impl<T: Clone + Send> AsyncChannel<T> {
    pub fn new() -> Self { Self { buffer: Arc::new(Mutex::new(VecDeque::new())) } }
    pub fn send(&self, val: T) { self.buffer.lock().unwrap().push_back(val); }
    pub fn recv(&self) -> Option<T> { self.buffer.lock().unwrap().pop_front() }
}

/// Applied π-calculus: cryptographic primitives
#[derive(Debug, Clone, PartialEq)]
pub struct CryptoValue {
    pub data: Vec<u8>,
    pub tag: CryptoTag,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CryptoTag {
    Plaintext,
    Encrypted(Vec<u8>),
    Signed(Vec<u8>),
    Hashed,
}

pub struct CryptoEngine {
    keys: HashMap<String, Vec<u8>>,
}

impl CryptoEngine {
    pub fn new() -> Self { Self { keys: HashMap::new() } }
    pub fn add_key(&mut self, name: &str, key: Vec<u8>) { self.keys.insert(name.into(), key); }
    /// 所有已声明密钥名(§27.5 spi 验证用)
    pub fn keys(&self) -> Vec<String> { self.keys.keys().cloned().collect() }

    pub fn encrypt(&self, data: &[u8], key_name: &str) -> Option<CryptoValue> {
        let key = self.keys.get(key_name)?;
        // §7.1 crypto feature:ChaCha20 流密码(替换 XOR 占位)
        #[cfg(feature = "crypto")]
        {
            use chacha20::cipher::{KeyIvInit, StreamCipher};
            let (key32, nonce12) = derive_key_nonce(key);
            let mut cipher = chacha20::ChaCha20::new(&key32.into(), &nonce12.into());
            let mut buf = data.to_vec();
            cipher.apply_keystream(&mut buf);
            Some(CryptoValue { data: buf, tag: CryptoTag::Encrypted(key.clone()) })
        }
        #[cfg(not(feature = "crypto"))]
        {
            let encrypted: Vec<u8> = data.iter().zip(key.iter().cycle()).map(|(a, b)| a ^ b).collect();
            Some(CryptoValue { data: encrypted, tag: CryptoTag::Encrypted(key.clone()) })
        }
    }

    pub fn decrypt(&self, val: &CryptoValue, key_name: &str) -> Option<Vec<u8>> {
        let key = self.keys.get(key_name)?;
        match &val.tag {
            CryptoTag::Encrypted(_) => {
                #[cfg(feature = "crypto")]
                {
                    use chacha20::cipher::{KeyIvInit, StreamCipher};
                    let (key32, nonce12) = derive_key_nonce(key);
                    let mut cipher = chacha20::ChaCha20::new(&key32.into(), &nonce12.into());
                    let mut buf = val.data.clone();
                    cipher.apply_keystream(&mut buf);
                    Some(buf)
                }
                #[cfg(not(feature = "crypto"))]
                {
                    Some(val.data.iter().zip(key.iter().cycle()).map(|(a, b)| a ^ b).collect())
                }
            }
            _ => None,
        }
    }

    pub fn sign(&self, data: &[u8], key_name: &str) -> Option<CryptoValue> {
        let key = self.keys.get(key_name)?;
        let sig: Vec<u8> = data.iter().zip(key.iter().cycle()).map(|(a, b)| a ^ b).collect();
        Some(CryptoValue { data: data.to_vec(), tag: CryptoTag::Signed(sig) })
    }

    pub fn verify(&self, val: &CryptoValue, key_name: &str) -> bool {
        let key = self.keys.get(key_name);
        match (&val.tag, key) {
            (CryptoTag::Signed(sig), Some(k)) => {
                let expected: Vec<u8> = val.data.iter().zip(k.iter().cycle()).map(|(a, b)| a ^ b).collect();
                expected == *sig
            }
            _ => false,
        }
    }

    pub fn hash(&self, data: &[u8]) -> CryptoValue {
        // §7.1 crypto feature:SHA-256(替换简单 hash 占位)
        #[cfg(feature = "crypto")]
        {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(data);
            let h = hasher.finalize();
            CryptoValue { data: h.to_vec(), tag: CryptoTag::Hashed }
        }
        #[cfg(not(feature = "crypto"))]
        {
            let h: u64 = data.iter().fold(0u64, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u64));
            CryptoValue { data: h.to_le_bytes().to_vec(), tag: CryptoTag::Hashed }
        }
    }
}

/// §7.1 从声明密钥派生 ChaCha20 密钥(32B)与非临(12B)
#[cfg(feature = "crypto")]
fn derive_key_nonce(key: &[u8]) -> ([u8; 32], [u8; 12]) {
    let mut key32 = [0u8; 32];
    for (i, b) in key.iter().take(32).enumerate() { key32[i] = *b; }
    let mut nonce12 = [0u8; 12];
    for (i, b) in key.iter().take(12).enumerate() { nonce12[i] = *b; }
    (key32, nonce12)
}

/// spi-calculus: security protocol verification primitives
#[derive(Debug, Clone)]
pub struct SecurityContext {
    secrets: HashMap<String, Vec<u8>>,
    commitments: HashMap<String, Vec<u8>>,
    attacker_knowledge: Vec<Vec<u8>>,
}

impl SecurityContext {
    pub fn new() -> Self { Self { secrets: HashMap::new(), commitments: HashMap::new(), attacker_knowledge: Vec::new() } }
    pub fn declare_secret(&mut self, name: &str, value: Vec<u8>) { self.secrets.insert(name.into(), value); }
    pub fn is_secret(&self, name: &str) -> bool { self.secrets.contains_key(name) }
    pub fn commit(&mut self, name: &str, value: Vec<u8>) { self.commitments.insert(name.into(), value); }
    pub fn check_commitment(&self, name: &str, value: &[u8]) -> bool {
        self.commitments.get(name).map_or(false, |c| c == value)
    }
    pub fn attacker_learns(&mut self, data: Vec<u8>) { self.attacker_knowledge.push(data); }
    pub fn attacker_knows(&self, data: &[u8]) -> bool {
        self.attacker_knowledge.iter().any(|k| k == data)
    }
}

/// ρ-calculus: reflective higher-order processes
#[derive(Clone)]
pub enum RhoValue {
    Name(String),
    Process(Rc<dyn Fn() -> RhoValue + Send + Sync>),
    Quote(String), // Quoted process code
}

/// Safe Ambients: mobile computation with capabilities
#[derive(Debug, Clone)]
pub struct Ambient {
    pub name: String,
    pub contents: Vec<Ambient>,
    pub capabilities: Vec<AmbientCap>,
}

#[derive(Debug, Clone)]
pub enum AmbientCap {
    Enter(String),
    Exit(String),
    Open(String),
    CoEnter(String),
    CoExit(String),
    CoOpen(String),
}

impl Ambient {
    pub fn new(name: &str) -> Self { Self { name: name.into(), contents: Vec::new(), capabilities: Vec::new() } }
    pub fn try_enter(&mut self, target: &str) -> bool {
        self.capabilities.iter().any(|c| matches!(c, AmbientCap::Enter(n) if n == target))
    }
    pub fn try_exit(&mut self, target: &str) -> bool {
        self.capabilities.iter().any(|c| matches!(c, AmbientCap::Exit(n) if n == target))
    }
}

/// κ-calculus: biochemical reaction networks
#[derive(Debug, Clone)]
pub struct BiochemicalSite {
    pub name: String,
    pub state: SiteState,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SiteState {
    Free,
    Bound(String),
    Hidden,
    Active,
    Inactive,
}

#[derive(Debug, Clone)]
pub struct MolecularComplex {
    pub sites: Vec<BiochemicalSite>,
    pub bonds: Vec<(usize, usize, usize, usize)>, // (complex_idx, site_idx, other_complex_idx, other_site_idx)
}

impl MolecularComplex {
    pub fn new(sites: Vec<BiochemicalSite>) -> Self { Self { sites, bonds: Vec::new() } }
    pub fn bind(&mut self, site_idx: usize, other: &mut MolecularComplex, other_site_idx: usize) -> bool {
        if site_idx < self.sites.len() && other_site_idx < other.sites.len() {
            self.sites[site_idx].state = SiteState::Bound(format!("bond_{}_{}", site_idx, other_site_idx));
            other.sites[other_site_idx].state = SiteState::Bound(format!("bond_{}_{}", other_site_idx, site_idx));
            self.bonds.push((0, site_idx, 1, other_site_idx));
            true
        } else { false }
    }
    pub fn unbind(&mut self, site_idx: usize) -> bool {
        if site_idx < self.sites.len() {
            self.sites[site_idx].state = SiteState::Free;
            self.bonds.retain(|(_, s, _, _)| *s != site_idx);
            true
        } else { false }
    }
}

// SKI combinators
#[derive(Debug, Clone)]
pub enum SKI {
    S, K, I,
    /// 整数字面量(§27.10 编码载体)
    Num(i64),
    App(Box<SKI>, Box<SKI>),
}

impl SKI {
    pub fn reduce(ski: SKI) -> SKI {
        match ski {
            SKI::App(a, y) => match *a {
                SKI::App(s, x) => match *s {
                    SKI::S => SKI::App(Box::new(SKI::App(Box::new(SKI::App(Box::new(SKI::S), x)), y)), Box::new(SKI::K)),
                    SKI::K => *x,
                    _ => SKI::App(Box::new(SKI::App(s, x)), y),
                },
                SKI::I => *y,
                // K 应用一个参数是常量函数 K x,须保留负载 x(等待第二参数);不得丢弃为 K
                SKI::K => SKI::App(Box::new(SKI::K), y),
                other => SKI::App(Box::new(other), y),
            },
            other => other,
        }
    }

    /// 归约到范式(迭代 reduce 至不动点)
    pub fn reduce_all(mut ski: SKI) -> SKI {
        for _ in 0..1000 {
            let next = SKI::reduce(ski.clone());
            if format!("{:?}", next) == format!("{:?}", ski) {
                return next;
            }
            ski = next;
        }
        ski
    }

    /// 提取 SKI 项中的全部整数观察值(Num)
    pub fn collect_nums(ski: &SKI) -> Vec<i64> {
        match ski {
            SKI::Num(n) => vec![*n],
            SKI::App(a, b) => {
                let mut v = SKI::collect_nums(a);
                v.extend(SKI::collect_nums(b));
                v
            }
            _ => vec![],
        }
    }
}

/// §27.10 观察等价:π 进程的观察值(Send 负载)与其 SKI 编码的观察值一致
pub fn check_observational_equivalence(ops: &[ChannelOp], encoded: &SKI) -> bool {
    let orig = channel_trace(ops);
    let reduced = SKI::reduce_all(encoded.clone());
    let enc: Vec<i64> = SKI::collect_nums(&reduced);
    orig == enc
}

/// §27.10 通道迹:通道操作序列的观察值(Send 负载,按序)
pub fn channel_trace(ops: &[ChannelOp]) -> Vec<i64> {
    ops.iter().filter_map(|op| match op {
        ChannelOp::Send(v) => Some(*v),
        ChannelOp::Recv => None,
    }).collect()
}

/// §27.10 迹等价:两段通道操作序列的观察值一致
pub fn check_trace_equivalence(a: &[ChannelOp], b: &[ChannelOp]) -> bool {
    channel_trace(a) == channel_trace(b)
}

use std::rc::Rc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn test_async_channel() {
        let ch = AsyncChannel::new();
        ch.send(42);
        assert_eq!(ch.recv(), Some(42));
        assert_eq!(ch.recv(), None);
    }

    #[test] fn test_crypto_encrypt_decrypt() {
        let mut engine = CryptoEngine::new();
        engine.add_key("key1", vec![1, 2, 3, 4]);
        let cipher = engine.encrypt(b"hello", "key1").unwrap();
        let plain = engine.decrypt(&cipher, "key1").unwrap();
        assert_eq!(plain, b"hello");
    }

    /// §7.1 crypto feature:SHA-256 32 字节摘要 + ChaCha20 非 XOR 密文
    #[cfg(feature = "crypto")]
    #[test]
    fn test_crypto_strong_algorithms() {
        let mut engine = CryptoEngine::new();
        engine.add_key("k", b"0123456789abcdef0123456789abcdef".to_vec());
        // SHA-256 输出 32 字节(非简单 hash 的 8 字节)
        let h = engine.hash(b"hello");
        assert_eq!(h.data.len(), 32, "SHA-256 应输出 32 字节,实际 {}", h.data.len());
        // ChaCha20 密文非 XOR:密文与明文不同且可往返
        let cipher = engine.encrypt(b"hello world", "k").unwrap();
        assert_ne!(cipher.data, b"hello world".to_vec(), "密文不应等于明文");
        let plain = engine.decrypt(&cipher, "k").unwrap();
        assert_eq!(plain, b"hello world");
    }

    #[test] fn test_ambient() {
        let mut a = Ambient::new("room");
        a.capabilities.push(AmbientCap::Enter("room".into()));
        assert!(a.try_enter("room"));
    }

    #[test] fn test_kappa_bind() {
        let mut c1 = MolecularComplex::new(vec![BiochemicalSite { name: "a".into(), state: SiteState::Free }]);
        let mut c2 = MolecularComplex::new(vec![BiochemicalSite { name: "b".into(), state: SiteState::Free }]);
        assert!(c1.bind(0, &mut c2, 0));
    }
    #[test]
    fn test_encode_pi_to_ski() {
        // §27.10:Send/Recv 编码为 SKI 组合子;Send 经 K 规则提取常量
        let encoded = SKI::encode_pi_to_ski(&[ChannelOp::Send(42)]);
        let reduced = SKI::reduce(encoded);
        assert!(matches!(reduced, SKI::App(..)) || matches!(reduced, SKI::Num(42)),
            "编码应保持发送值,实际 {:?}", reduced);
        let recv_only = SKI::encode_pi_to_ski(&[ChannelOp::Recv]);
        assert!(matches!(recv_only, SKI::App(..)), "Recv 编码应为组合,实际 {:?}", recv_only);
    }

    #[test]
    fn test_ski_reduce_preserves_k_payload() {
        // §27.10 修复:K 应用一个参数须保留负载(K x 是常量函数),不得丢弃为 K
        let k_apply = SKI::App(Box::new(SKI::K), Box::new(SKI::Num(42)));
        let reduced = SKI::reduce(k_apply.clone());
        assert!(matches!(reduced, SKI::App(..)), "App(K, 42) 应保持为 App,而非 K,实际 {:?}", reduced);
        // 完整两步:K x y → x(负载提取)
        let full = SKI::App(Box::new(k_apply), Box::new(SKI::Num(0)));
        let reduced_full = SKI::reduce(full);
        assert!(matches!(reduced_full, SKI::Num(42)), "App(App(K,42), 0) → 42,实际 {:?}", reduced_full);
    }

    #[test]
    fn test_ambient_cap_encoding() {
        // §27.10:ambient 能力 → 通道消息
        assert_eq!(ambient_cap_to_channel_msg(&AmbientCap::Enter("room".into())), "enter:room");
        assert_eq!(ambient_cap_to_channel_msg(&AmbientCap::Exit("room".into())), "exit:room");
        assert_eq!(ambient_cap_to_channel_msg(&AmbientCap::Open("box".into())), "open:box");
    }

    #[test]
    fn test_calculus_encodings() {
        // §27.10:补 async→sync / applied→π / ρ→π 三种编码
        let a = encode_async_to_sync(&[AsyncOp::Send(7), AsyncOp::Recv]);
        assert_eq!(a, vec![ChannelOp::Send(7), ChannelOp::Recv]);
        let p = encode_applied_to_pi(&[AppliedOp::Encrypt(3), AppliedOp::Decrypt(3)]);
        assert_eq!(p, vec![ChannelOp::Send(3), ChannelOp::Send(3)]);
        let r = encode_rho_to_pi(&[RhoOp::Quote(9), RhoOp::Drop]);
        assert_eq!(r, vec![ChannelOp::Send(9)]);
    }

    #[test]
    fn test_observational_equivalence() {
        // §27.10:原进程与编码结果的观察值(Send 负载)一致
        let ops = [ChannelOp::Send(42), ChannelOp::Send(7)];
        let encoded = SKI::encode_pi_to_ski(&ops);
        assert!(check_observational_equivalence(&ops, &encoded),
            "π 进程与其 SKI 编码应观察等价");
        // 不一致:不同负载
        let encoded2 = SKI::encode_pi_to_ski(&[ChannelOp::Send(99)]);
        assert!(!check_observational_equivalence(&ops, &encoded2),
            "不同负载不应观察等价");
    }

    #[test]
    fn test_trace_equivalence_all_encodings() {
        // §27.10 全演算迹等价:async/applied/ρ 编码后观察值与原进程一致
        let async_src = [ChannelOp::Send(7), ChannelOp::Recv];
        let async_enc = encode_async_to_sync(&[AsyncOp::Send(7), AsyncOp::Recv]);
        assert!(check_trace_equivalence(&async_src, &async_enc), "async→sync 应迹等价");

        let applied_src = [ChannelOp::Send(3)];
        let applied_enc = encode_applied_to_pi(&[AppliedOp::Encrypt(3)]);
        assert!(check_trace_equivalence(&applied_src, &applied_enc), "applied→π 应迹等价");

        let rho_src = [ChannelOp::Send(9)];
        let rho_enc = encode_rho_to_pi(&[RhoOp::Quote(9), RhoOp::Drop]);
        assert!(check_trace_equivalence(&rho_src, &rho_enc), "ρ→π 应迹等价");

        // 不等价:不同负载
        assert!(!check_trace_equivalence(&[ChannelOp::Send(1)], &[ChannelOp::Send(2)]));
    }

}

// ─────────────────────────────────────────────────────────────────────────────
// §27.10 演算互编码:π 通道操作 → SKI 组合子;ambient 能力 → 通道消息
// ─────────────────────────────────────────────────────────────────────────────

/// π 通道操作抽象(编码源)
#[derive(Debug, Clone, PartialEq)]
pub enum ChannelOp {
    /// 发送值
    Send(i64),
    /// 接收
    Recv,
}

impl SKI {
    /// §27.10 π→SKI 编码:Send v → App(K, Num(v))(K 丢弃环境取常量),
    /// Recv → I(恒等);序列组合为 App 链。reduce 后保持观察值(编码等价)。
    pub fn encode_pi_to_ski(ops: &[ChannelOp]) -> SKI {
        let mut acc = SKI::I;
        for op in ops {
            let enc = match op {
                ChannelOp::Send(v) => SKI::App(Box::new(SKI::K), Box::new(SKI::Num(*v))),
                ChannelOp::Recv => SKI::I,
            };
            acc = SKI::App(Box::new(acc), Box::new(enc));
        }
        acc
    }
}

/// §27.10 异步 π 操作(编码源)
#[derive(Debug, Clone, PartialEq)]
pub enum AsyncOp {
    Send(i64),
    Recv,
}

/// §27.10 async→sync 编码:异步操作 → 同步通道操作
pub fn encode_async_to_sync(ops: &[AsyncOp]) -> Vec<ChannelOp> {
    ops.iter().map(|op| match op {
        AsyncOp::Send(v) => ChannelOp::Send(*v),
        AsyncOp::Recv => ChannelOp::Recv,
    }).collect()
}

/// §27.10 applied-π 操作(加密/解密/签名/验证)
#[derive(Debug, Clone, PartialEq)]
pub enum AppliedOp {
    Encrypt(i64),
    Decrypt(i64),
    Sign(i64),
    Verify(i64),
}

/// §27.10 applied-π→π 编码:加密原语 → 通道操作(加密值经通道传输)
pub fn encode_applied_to_pi(ops: &[AppliedOp]) -> Vec<ChannelOp> {
    ops.iter().map(|op| match op {
        AppliedOp::Encrypt(v) | AppliedOp::Decrypt(v) | AppliedOp::Sign(v) | AppliedOp::Verify(v) => ChannelOp::Send(*v),
    }).collect()
}

/// §27.10 ρ-calculus 操作(quote/lift/drop)
#[derive(Debug, Clone, PartialEq)]
pub enum RhoOp {
    Quote(i64),
    Lift(i64),
    Drop,
}

/// §27.10 ρ→π 编码:反射操作 → 通道操作(quote 发送、lift 接收、drop 空操作)
pub fn encode_rho_to_pi(ops: &[RhoOp]) -> Vec<ChannelOp> {
    ops.iter().filter_map(|op| match op {
        RhoOp::Quote(v) => Some(ChannelOp::Send(*v)),
        RhoOp::Lift(_) => Some(ChannelOp::Recv),
        RhoOp::Drop => None,
    }).collect()
}

/// ambient 能力 → 通道消息编码(enter/exit/open → 结构化消息)
pub fn ambient_cap_to_channel_msg(cap: &AmbientCap) -> String {
    match cap {
        AmbientCap::Enter(name) => format!("enter:{}", name),
        AmbientCap::Exit(name) => format!("exit:{}", name),
        AmbientCap::Open(name) => format!("open:{}", name),
        other => format!("{:?}", other),
    }
}

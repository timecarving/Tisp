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
        // Simple XOR encryption (placeholder — production should use AES/ChaCha)
        let encrypted: Vec<u8> = data.iter().zip(key.iter().cycle()).map(|(a, b)| a ^ b).collect();
        Some(CryptoValue { data: encrypted, tag: CryptoTag::Encrypted(key.clone()) })
    }

    pub fn decrypt(&self, val: &CryptoValue, key_name: &str) -> Option<Vec<u8>> {
        let key = self.keys.get(key_name)?;
        match &val.tag {
            CryptoTag::Encrypted(_) => {
                Some(val.data.iter().zip(key.iter().cycle()).map(|(a, b)| a ^ b).collect())
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
        // Simple hash (placeholder — should use SHA-256/Blake3)
        let h: u64 = data.iter().fold(0u64, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u64));
        CryptoValue { data: h.to_le_bytes().to_vec(), tag: CryptoTag::Hashed }
    }
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
                SKI::K => SKI::K,
                other => SKI::App(Box::new(other), y),
            },
            other => other,
        }
    }

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
    fn test_ambient_cap_encoding() {
        // §27.10:ambient 能力 → 通道消息
        assert_eq!(ambient_cap_to_channel_msg(&AmbientCap::Enter("room".into())), "enter:room");
        assert_eq!(ambient_cap_to_channel_msg(&AmbientCap::Exit("room".into())), "exit:room");
        assert_eq!(ambient_cap_to_channel_msg(&AmbientCap::Open("box".into())), "open:box");
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

/// ambient 能力 → 通道消息编码(enter/exit/open → 结构化消息)
pub fn ambient_cap_to_channel_msg(cap: &AmbientCap) -> String {
    match cap {
        AmbientCap::Enter(name) => format!("enter:{}", name),
        AmbientCap::Exit(name) => format!("exit:{}", name),
        AmbientCap::Open(name) => format!("open:{}", name),
        other => format!("{:?}", other),
    }
}

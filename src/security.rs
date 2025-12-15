//! Security module for TrinityChain
//!
//! Provides peer authentication, firewall rules, rate limiting, and VPN support

use crate::crypto::KeyPair;
use crate::error::ChainError;
use ipnetwork::IpNetwork;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Peer authentication handshake challenge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerChallenge {
    /// Challenge nonce (random bytes)
    pub nonce: String,
    /// Node public key (hex encoded)
    pub public_key: String,
    /// Timestamp of challenge
    pub timestamp: u64,
}

/// Peer authentication response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerAuthResponse {
    /// Signed challenge nonce
    pub signature: String,
    /// Public key of responder
    pub public_key: String,
    /// Timestamp
    pub timestamp: u64,
    /// Protocol version
    pub version: String,
}

/// Peer identity and reputation
#[derive(Debug, Clone)]
pub struct PeerIdentity {
    pub address: String,
    pub public_key: Vec<u8>,
    pub authenticated: bool,
    pub failed_attempts: u32,
    pub last_seen: u64,
}

impl PeerIdentity {
    /// Check if peer is trusted (authenticated and not too many failed attempts)
    pub fn is_trusted(&self) -> bool {
        self.authenticated && self.failed_attempts < 3
    }

    /// Record a failed authentication attempt
    pub fn record_failure(&mut self) {
        self.failed_attempts += 1;
    }

    /// Record successful authentication
    pub fn mark_authenticated(&mut self) {
        self.authenticated = true;
        self.failed_attempts = 0;
        self.last_seen = current_timestamp();
    }
}

/// Firewall rule for network access control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FirewallRule {
    /// Allow connections from IP/network
    Allow(IpNetwork),
    /// Deny connections from IP/network
    Deny(IpNetwork),
}

/// Network policy and firewall
#[derive(Debug, Clone)]
pub struct NetworkPolicy {
    /// List of firewall rules (evaluated in order)
    rules: Vec<FirewallRule>,
    /// Require peer authentication
    require_auth: bool,
    /// VPN tunnel interface name (if using VPN)
    vpn_interface: Option<String>,
    /// SOCKS5 proxy for outbound connections
    socks5_proxy: Option<String>,
}

impl Default for NetworkPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkPolicy {
    /// Create a new network policy
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            require_auth: true,
            vpn_interface: None,
            socks5_proxy: None,
        }
    }

    /// Load policy from environment variables
    pub fn from_env() -> Self {
        let mut policy = Self::new();

        // Check for VPN tunnel
        if let Ok(interface) = std::env::var("TRINITY_VPN_INTERFACE") {
            policy.vpn_interface = Some(interface);
        }

        // Check for SOCKS5 proxy
        if let Ok(proxy) = std::env::var("TRINITY_SOCKS5_PROXY") {
            policy.socks5_proxy = Some(proxy);
        }

        // Check if auth is required
        if let Ok(auth) = std::env::var("TRINITY_REQUIRE_AUTH") {
            policy.require_auth = auth.to_lowercase() != "false";
        }

        policy
    }

    /// Add a firewall rule
    pub fn add_rule(&mut self, rule: FirewallRule) {
        self.rules.push(rule);
    }

    /// Check if IP address is allowed by firewall rules
    pub fn is_ip_allowed(&self, ip: IpAddr) -> bool {
        // If no rules, allow by default
        if self.rules.is_empty() {
            return true;
        }

        // Evaluate rules in order
        for rule in &self.rules {
            match rule {
                FirewallRule::Allow(network) => {
                    if network.contains(ip) {
                        return true;
                    }
                }
                FirewallRule::Deny(network) => {
                    if network.contains(ip) {
                        return false;
                    }
                }
            }
        }

        // Default: deny if no matching allow rule
        false
    }

    /// Get VPN interface if configured
    pub fn get_vpn_interface(&self) -> Option<&str> {
        self.vpn_interface.as_deref()
    }

    /// Get SOCKS5 proxy if configured
    pub fn get_socks5_proxy(&self) -> Option<&str> {
        self.socks5_proxy.as_deref()
    }

    /// Check if peer authentication is required
    pub fn requires_auth(&self) -> bool {
        self.require_auth
    }
}

/// Rate limiter for peer connections and API requests
#[derive(Debug)]
pub struct RateLimitConfig {
    /// Per-peer request limit (requests per second)
    pub peer_requests_per_sec: u32,
    /// API requests per IP (requests per second)
    pub api_requests_per_sec: u32,
    /// Transaction submission rate (transactions per second)
    pub transactions_per_sec: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            peer_requests_per_sec: 100,
            api_requests_per_sec: 50,
            transactions_per_sec: 10,
        }
    }
}

/// Token bucket rate limiter entry
#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    last_refill: u64,
    rate: u32,
}

impl TokenBucket {
    fn new(rate: u32) -> Self {
        Self {
            tokens: rate as f64,
            last_refill: current_timestamp(),
            rate,
        }
    }

    fn try_consume(&mut self) -> bool {
        let now = current_timestamp();
        let elapsed = now.saturating_sub(self.last_refill);

        // Refill tokens based on elapsed time
        self.tokens += (elapsed as f64) * (self.rate as f64);
        self.last_refill = now;

        // Cap tokens at rate
        if self.tokens > self.rate as f64 {
            self.tokens = self.rate as f64;
        }

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Per-entity rate limiter
#[derive(Debug)]
pub struct RequestRateLimiter {
    // Map of entity ID to token bucket
    limiters: Arc<RwLock<HashMap<String, TokenBucket>>>,
    config: RateLimitConfig,
}

impl RequestRateLimiter {
    /// Create a new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            limiters: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Check if peer request is allowed
    pub fn check_peer_rate_limit(&self, peer_id: &str) -> Result<(), ChainError> {
        self.check_rate_limit(
            peer_id,
            self.config.peer_requests_per_sec,
            "peer request limit exceeded",
        )
    }

    /// Check if API request is allowed
    pub fn check_api_rate_limit(&self, client_ip: &str) -> Result<(), ChainError> {
        self.check_rate_limit(
            client_ip,
            self.config.api_requests_per_sec,
            "API rate limit exceeded",
        )
    }

    /// Check if transaction rate is allowed
    pub fn check_transaction_rate_limit(&self, wallet_addr: &str) -> Result<(), ChainError> {
        self.check_rate_limit(
            wallet_addr,
            self.config.transactions_per_sec,
            "transaction rate limit exceeded",
        )
    }

    /// Internal rate limit check using token bucket algorithm
    fn check_rate_limit(
        &self,
        entity_id: &str,
        rate: u32,
        error_msg: &str,
    ) -> Result<(), ChainError> {
        let mut limiters = self.limiters.write();

        let bucket = limiters
            .entry(entity_id.to_string())
            .or_insert_with(|| TokenBucket::new(rate));

        if bucket.try_consume() {
            Ok(())
        } else {
            Err(ChainError::NetworkError(error_msg.to_string()))
        }
    }
}

/// Security manager combining all security features
#[derive(Debug)]
pub struct SecurityManager {
    /// Network policy and firewall
    network_policy: Arc<RwLock<NetworkPolicy>>,
    /// Peer identities and reputations
    peers: Arc<RwLock<HashMap<String, PeerIdentity>>>,
    /// Rate limiters
    rate_limiter: Arc<RequestRateLimiter>,
    /// Local node keypair for authentication
    node_keypair: KeyPair,
}

impl SecurityManager {
    /// Create a new security manager
    pub fn new(node_keypair: KeyPair) -> Result<Self, ChainError> {
        let policy = NetworkPolicy::from_env();

        Ok(Self {
            network_policy: Arc::new(RwLock::new(policy)),
            peers: Arc::new(RwLock::new(HashMap::new())),
            rate_limiter: Arc::new(RequestRateLimiter::new(RateLimitConfig::default())),
            node_keypair,
        })
    }

    /// Create authentication challenge for peer
    pub fn create_challenge(&self) -> Result<PeerChallenge, ChainError> {
        let mut nonce_bytes = [0u8; 32];
        use rand::RngCore;
        rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);

        Ok(PeerChallenge {
            nonce: hex::encode(nonce_bytes),
            public_key: hex::encode(self.node_keypair.public_key_bytes()),
            timestamp: current_timestamp(),
        })
    }

    /// Respond to authentication challenge
    pub fn create_auth_response(
        &self,
        challenge: &PeerChallenge,
    ) -> Result<PeerAuthResponse, ChainError> {
        let signature = self.node_keypair.sign(challenge.nonce.as_bytes())?;

        Ok(PeerAuthResponse {
            signature: hex::encode(signature),
            public_key: hex::encode(self.node_keypair.public_key_bytes()),
            timestamp: current_timestamp(),
            version: "1.0".to_string(),
        })
    }

    /// Verify peer authentication response
    pub fn verify_auth_response(
        &self,
        peer_id: &str,
        challenge: &PeerChallenge,
        response: &PeerAuthResponse,
    ) -> Result<(), ChainError> {
        // Verify timestamp is recent (within 5 minutes)
        let now = current_timestamp();
        if now.saturating_sub(response.timestamp) > 300 {
            return Err(ChainError::AuthenticationError(
                "Challenge response expired".to_string(),
            ));
        }

        // Verify signature
        let public_key_bytes = hex::decode(&response.public_key)
            .map_err(|e| ChainError::AuthenticationError(format!("Invalid public key: {}", e)))?;

        let signature = hex::decode(&response.signature)
            .map_err(|e| ChainError::AuthenticationError(format!("Invalid signature: {}", e)))?;

        crate::crypto::verify_signature(&public_key_bytes, challenge.nonce.as_bytes(), &signature)?;

        // Mark peer as authenticated
        let mut peers = self.peers.write();
        let peer_entry = peers
            .entry(peer_id.to_string())
            .or_insert_with(|| PeerIdentity {
                address: peer_id.to_string(),
                public_key: public_key_bytes.clone(),
                authenticated: false,
                failed_attempts: 0,
                last_seen: now,
            });

        peer_entry.mark_authenticated();

        Ok(())
    }

    /// Check if peer is allowed to connect
    pub fn check_peer_allowed(&self, peer_addr: &str) -> Result<(), ChainError> {
        // Parse IP from address (format: "ip:port")
        let ip_str = peer_addr.split(':').next().ok_or_else(|| {
            ChainError::AuthenticationError("Invalid peer address format".to_string())
        })?;

        let ip: IpAddr = ip_str.parse().map_err(|_| {
            ChainError::AuthenticationError(format!("Invalid IP address: {}", ip_str))
        })?;

        // Check firewall rules
        let policy = self.network_policy.read();
        if !policy.is_ip_allowed(ip) {
            return Err(ChainError::NetworkError(
                "Peer IP blocked by firewall".to_string(),
            ));
        }

        // Check rate limit
        self.rate_limiter.check_peer_rate_limit(peer_addr)?;

        Ok(())
    }

    /// Check API request rate limit
    pub fn check_api_limit(&self, client_ip: &str) -> Result<(), ChainError> {
        self.rate_limiter.check_api_rate_limit(client_ip)
    }

    /// Check transaction rate limit
    pub fn check_transaction_limit(&self, wallet_addr: &str) -> Result<(), ChainError> {
        self.rate_limiter.check_transaction_rate_limit(wallet_addr)
    }

    /// Get network policy
    pub fn network_policy(&self) -> Arc<RwLock<NetworkPolicy>> {
        Arc::clone(&self.network_policy)
    }

    /// Get peer list
    pub fn get_peers(&self) -> HashMap<String, PeerIdentity> {
        self.peers.read().clone()
    }
}

/// Get current Unix timestamp
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_policy_creation() {
        let policy = NetworkPolicy::new();
        assert!(policy.requires_auth());
    }

    #[test]
    fn test_firewall_rules() {
        let mut policy = NetworkPolicy::new();

        // Allow localhost
        policy.add_rule(FirewallRule::Allow(
            "127.0.0.1/8".parse().expect("Failed to parse IP network"),
        ));

        // Allow private network
        policy.add_rule(FirewallRule::Allow(
            "192.168.0.0/16"
                .parse()
                .expect("Failed to parse IP network"),
        ));

        // Check IP addresses
        assert!(policy.is_ip_allowed("127.0.0.1".parse().expect("Failed to parse IP")));
        assert!(policy.is_ip_allowed("192.168.1.1".parse().expect("Failed to parse IP")));
    }

    #[test]
    fn test_rate_limiter() {
        let limiter = RequestRateLimiter::new(RateLimitConfig {
            peer_requests_per_sec: 10,
            api_requests_per_sec: 10,
            transactions_per_sec: 5,
        });

        // Should allow first request
        assert!(limiter.check_peer_rate_limit("peer1").is_ok());
    }
}

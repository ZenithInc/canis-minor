use std::{
    fs,
    net::{Ipv4Addr, SocketAddrV4},
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;

use crate::DEFAULT_MAX_STREAMS;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    pub listen: SocketAddrV4,
    pub psk_file: PathBuf,
    #[serde(default = "default_dns_upstream")]
    pub dns_upstream: SocketAddrV4,
    #[serde(default = "default_handshake_timeout_secs")]
    pub handshake_timeout_secs: u64,
    #[serde(default = "default_connect_timeout_secs")]
    pub connect_timeout_secs: u64,
    #[serde(default = "default_udp_idle_secs")]
    pub udp_idle_secs: u64,
    #[serde(default = "default_heartbeat_secs")]
    pub heartbeat_secs: u64,
    #[serde(default = "default_max_streams")]
    pub max_streams: usize,
}

impl ServerConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)
            .with_context(|| format!("failed to read server config {}", path.display()))?;
        let config: Self = toml::from_str(&text)
            .with_context(|| format!("invalid server config {}", path.display()))?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(self.listen.port() != 0, "listen port must not be zero");
        ensure!(
            self.dns_upstream.port() == 53,
            "dns_upstream must use port 53"
        );
        validate_common(
            self.handshake_timeout_secs,
            self.heartbeat_secs,
            self.max_streams,
        )?;
        ensure!(
            self.connect_timeout_secs > 0,
            "connect timeout must be positive"
        );
        ensure!(self.udp_idle_secs > 0, "UDP idle timeout must be positive");
        Ok(())
    }

    pub fn handshake_timeout(&self) -> Duration {
        Duration::from_secs(self.handshake_timeout_secs)
    }

    pub fn connect_timeout(&self) -> Duration {
        Duration::from_secs(self.connect_timeout_secs)
    }

    pub fn udp_idle_timeout(&self) -> Duration {
        Duration::from_secs(self.udp_idle_secs)
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientConfig {
    pub server_addr: SocketAddrV4,
    pub psk_file: PathBuf,
    #[serde(default = "default_proxy_port")]
    pub proxy_port: u16,
    #[serde(default = "default_fwmark")]
    pub fwmark: u32,
    #[serde(default = "default_bypass_mark")]
    pub bypass_mark: u32,
    #[serde(default = "default_route_table")]
    pub route_table: u32,
    #[serde(default = "default_rule_priority")]
    pub rule_priority: u32,
    #[serde(default = "default_nft_table")]
    pub nft_table: String,
    #[serde(default = "default_handshake_timeout_secs")]
    pub handshake_timeout_secs: u64,
    #[serde(default = "default_heartbeat_secs")]
    pub heartbeat_secs: u64,
    #[serde(default = "default_udp_idle_secs")]
    pub udp_idle_secs: u64,
    #[serde(default = "default_reconnect_min_secs")]
    pub reconnect_min_secs: u64,
    #[serde(default = "default_reconnect_max_secs")]
    pub reconnect_max_secs: u64,
    #[serde(default = "default_max_streams")]
    pub max_streams: usize,
}

impl ClientConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)
            .with_context(|| format!("failed to read client config {}", path.display()))?;
        let config: Self = toml::from_str(&text)
            .with_context(|| format!("invalid client config {}", path.display()))?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(self.server_addr.port() != 0, "server port must not be zero");
        ensure!(
            !self.server_addr.ip().is_unspecified(),
            "server address must not be unspecified"
        );
        ensure!(self.proxy_port != 0, "proxy port must not be zero");
        ensure!(self.fwmark != 0, "fwmark must not be zero");
        ensure!(self.bypass_mark != 0, "bypass_mark must not be zero");
        ensure!(
            self.fwmark != self.bypass_mark,
            "fwmark and bypass_mark must differ"
        );
        ensure!(self.route_table > 0, "route_table must be positive");
        ensure!(self.rule_priority > 0, "rule_priority must be positive");
        ensure!(
            !self.nft_table.is_empty()
                && self
                    .nft_table
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_'),
            "nft_table may contain only ASCII letters, digits, and underscores"
        );
        validate_common(
            self.handshake_timeout_secs,
            self.heartbeat_secs,
            self.max_streams,
        )?;
        ensure!(self.udp_idle_secs > 0, "UDP idle timeout must be positive");
        ensure!(
            self.reconnect_min_secs > 0,
            "minimum reconnect delay must be positive"
        );
        ensure!(
            self.reconnect_max_secs >= self.reconnect_min_secs,
            "maximum reconnect delay must not be less than the minimum"
        );
        Ok(())
    }

    pub fn handshake_timeout(&self) -> Duration {
        Duration::from_secs(self.handshake_timeout_secs)
    }

    pub fn udp_idle_timeout(&self) -> Duration {
        Duration::from_secs(self.udp_idle_secs)
    }
}

fn validate_common(handshake_timeout: u64, heartbeat: u64, max_streams: usize) -> Result<()> {
    ensure!(handshake_timeout > 0, "handshake timeout must be positive");
    ensure!(heartbeat > 0, "heartbeat interval must be positive");
    ensure!(
        (1..=DEFAULT_MAX_STREAMS).contains(&max_streams),
        "max_streams must be between 1 and {DEFAULT_MAX_STREAMS}"
    );
    Ok(())
}

pub fn load_psk(path: impl AsRef<Path>) -> Result<[u8; 32]> {
    let path = path.as_ref();
    let metadata = fs::metadata(path)
        .with_context(|| format!("failed to inspect PSK file {}", path.display()))?;
    ensure!(
        metadata.is_file(),
        "PSK path {} is not a file",
        path.display()
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            bail!(
                "PSK file {} is accessible by group or others (mode {mode:03o}); expected 0600",
                path.display()
            );
        }
    }

    let bytes =
        fs::read(path).with_context(|| format!("failed to read PSK file {}", path.display()))?;
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        anyhow::anyhow!(
            "PSK file {} must contain exactly 32 raw bytes, found {}",
            path.display(),
            bytes.len()
        )
    })
}

fn default_dns_upstream() -> SocketAddrV4 {
    SocketAddrV4::new(Ipv4Addr::new(1, 1, 1, 1), 53)
}

const fn default_handshake_timeout_secs() -> u64 {
    10
}

const fn default_connect_timeout_secs() -> u64 {
    10
}

const fn default_udp_idle_secs() -> u64 {
    60
}

const fn default_heartbeat_secs() -> u64 {
    5
}

const fn default_max_streams() -> usize {
    DEFAULT_MAX_STREAMS
}

const fn default_proxy_port() -> u16 {
    15260
}

const fn default_fwmark() -> u32 {
    0x00ca_5260
}

const fn default_bypass_mark() -> u32 {
    0x00ca_5261
}

const fn default_route_table() -> u32 {
    5260
}

const fn default_rule_priority() -> u32 {
    10526
}

fn default_nft_table() -> String {
    "canis_minor".to_owned()
}

const fn default_reconnect_min_secs() -> u64 {
    1
}

const fn default_reconnect_max_secs() -> u64 {
    30
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    fn valid_client() -> ClientConfig {
        ClientConfig {
            server_addr: "192.0.2.10:5260".parse().unwrap(),
            psk_file: "/tmp/key".into(),
            proxy_port: 15260,
            fwmark: 10,
            bypass_mark: 11,
            route_table: 5260,
            rule_priority: 10526,
            nft_table: "canis_minor".into(),
            handshake_timeout_secs: 10,
            heartbeat_secs: 5,
            udp_idle_secs: 60,
            reconnect_min_secs: 1,
            reconnect_max_secs: 30,
            max_streams: 256,
        }
    }

    #[test]
    fn client_config_validation_rejects_recursive_mark() {
        let mut config = valid_client();
        config.bypass_mark = config.fwmark;
        assert!(
            config
                .validate()
                .unwrap_err()
                .to_string()
                .contains("differ")
        );
    }

    #[test]
    fn client_config_validation_rejects_unsafe_table_name() {
        let mut config = valid_client();
        config.nft_table = "bad; delete table".into();
        assert!(config.validate().is_err());
    }

    #[test]
    fn server_defaults_include_cloudflare_dns() {
        let config: ServerConfig = toml::from_str(
            r#"
                listen = "0.0.0.0:5260"
                psk_file = "/tmp/key"
            "#,
        )
        .unwrap();
        assert_eq!(config.dns_upstream, "1.1.1.1:53".parse().unwrap());
        assert!(config.validate().is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn psk_requires_exact_length_and_private_permissions() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("psk");
        fs::write(&path, [9_u8; 32]).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(load_psk(&path).unwrap(), [9_u8; 32]);

        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(load_psk(&path).unwrap_err().to_string().contains("0600"));

        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        fs::write(&path, [9_u8; 31]).unwrap();
        assert!(
            load_psk(&path)
                .unwrap_err()
                .to_string()
                .contains("exactly 32")
        );
    }
}

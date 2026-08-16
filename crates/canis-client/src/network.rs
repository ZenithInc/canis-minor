use std::{io, process::Stdio};

use anyhow::{Context, Result, anyhow};
use canis_protocol::config::ClientConfig;
use tokio::{io::AsyncWriteExt, process::Command};
use tracing::{debug, warn};

#[derive(Clone, Debug)]
pub struct NetworkRules {
    config: ClientConfig,
}

impl NetworkRules {
    pub fn new(config: ClientConfig) -> Self {
        Self { config }
    }

    pub async fn install(&self) -> Result<()> {
        self.cleanup().await?;

        run_checked(
            "ip",
            &[
                "-4".into(),
                "route".into(),
                "add".into(),
                "local".into(),
                "0.0.0.0/0".into(),
                "dev".into(),
                "lo".into(),
                "table".into(),
                self.config.route_table.to_string(),
            ],
            None,
        )
        .await
        .context("failed to install transparent-proxy route")?;

        let rule_result = run_checked(
            "ip",
            &[
                "-4".into(),
                "rule".into(),
                "add".into(),
                "priority".into(),
                self.config.rule_priority.to_string(),
                "fwmark".into(),
                format!("{:#x}/0xffffffff", self.config.fwmark),
                "lookup".into(),
                self.config.route_table.to_string(),
            ],
            None,
        )
        .await;
        if let Err(error) = rule_result {
            self.cleanup().await?;
            return Err(error).context("failed to install transparent-proxy policy rule");
        }

        let script = self.nft_script();
        if let Err(error) = run_checked("nft", &["-f".into(), "-".into()], Some(&script)).await {
            self.cleanup().await?;
            return Err(error).context("failed to install nftables interception table");
        }
        Ok(())
    }

    pub async fn cleanup(&self) -> Result<()> {
        let mut first_error = None;
        for result in [
            run_tolerant(
                "nft",
                &[
                    "delete".into(),
                    "table".into(),
                    "inet".into(),
                    self.config.nft_table.clone(),
                ],
            )
            .await,
            run_tolerant(
                "ip",
                &[
                    "-4".into(),
                    "rule".into(),
                    "del".into(),
                    "priority".into(),
                    self.config.rule_priority.to_string(),
                    "fwmark".into(),
                    format!("{:#x}/0xffffffff", self.config.fwmark),
                    "lookup".into(),
                    self.config.route_table.to_string(),
                ],
            )
            .await,
            run_tolerant(
                "ip",
                &[
                    "-4".into(),
                    "route".into(),
                    "del".into(),
                    "local".into(),
                    "0.0.0.0/0".into(),
                    "dev".into(),
                    "lo".into(),
                    "table".into(),
                    self.config.route_table.to_string(),
                ],
            )
            .await,
        ] {
            if let Err(error) = result {
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    pub fn nft_script(&self) -> String {
        let server_ip = self.config.server_addr.ip();
        let server_port = self.config.server_addr.port();
        let mark = self.config.fwmark;
        let bypass = self.config.bypass_mark;
        let proxy_port = self.config.proxy_port;
        let table = &self.config.nft_table;

        format!(
            r#"table inet {table} {{
    chain output {{
        type route hook output priority mangle; policy accept;
        meta mark {bypass:#x} return
        ct direction reply return
        ip daddr {server_ip} tcp dport {server_port} return
        ip daddr {{ 0.0.0.0/8, 127.0.0.0/8, 169.254.0.0/16, 224.0.0.0/3 }} return
        udp sport 67 udp dport 68 return
        udp sport 68 udp dport 67 return
        udp dport 53 ct mark set {mark:#x} meta mark set {mark:#x}
        tcp dport 53 ct mark set {mark:#x} meta mark set {mark:#x}
        ct state established ct mark != {mark:#x} return
        meta l4proto {{ tcp, udp }} ct mark set {mark:#x} meta mark set {mark:#x}
    }}

    chain prerouting {{
        type filter hook prerouting priority mangle; policy accept;
        meta mark {mark:#x} meta l4proto {{ tcp, udp }} tproxy to :{proxy_port} accept
    }}
}}
"#
        )
    }
}

pub async fn flush_resolved_cache() {
    match Command::new("resolvectl")
        .arg("flush-caches")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .await
    {
        Ok(output) if output.status.success() => debug!("systemd-resolved cache flushed"),
        Ok(output) => warn!(
            error = %String::from_utf8_lossy(&output.stderr).trim(),
            "failed to flush systemd-resolved cache"
        ),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            debug!("resolvectl is not installed; skipping DNS cache flush")
        }
        Err(error) => warn!(%error, "failed to run resolvectl"),
    }
}

async fn run_checked(program: &str, args: &[String], stdin: Option<&str>) -> Result<()> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .with_context(|| format!("failed to execute {program}"))?;
    if let Some(input) = stdin {
        child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("failed to open {program} stdin"))?
            .write_all(input.as_bytes())
            .await?;
    }
    let output = child.wait_with_output().await?;
    if !output.status.success() {
        return Err(anyhow!(
            "{program} exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

async fn run_tolerant(program: &str, args: &[String]) -> Result<()> {
    match Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
    {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Err(anyhow!("required command {program} is not installed"))
        }
        Err(error) => Err(error).with_context(|| format!("failed to execute {program}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> ClientConfig {
        toml::from_str(
            r#"
                server_addr = "192.0.2.25:5260"
                psk_file = "/tmp/key"
                proxy_port = 15260
                fwmark = 13259360
                bypass_mark = 13259361
                route_table = 5260
                rule_priority = 10526
                nft_table = "canis_minor"
            "#,
        )
        .unwrap()
    }

    #[test]
    fn rules_exclude_tunnel_replies_dhcp_and_non_unicast() {
        let rules = NetworkRules::new(config()).nft_script();
        assert!(rules.contains("ct direction reply return"));
        assert!(rules.contains("ip daddr 192.0.2.25 tcp dport 5260 return"));
        assert!(rules.contains("udp sport 67 udp dport 68 return"));
        assert!(rules.contains("udp dport 53 ct mark set"));
        assert!(rules.contains("tcp dport 53 ct mark set"));
        assert!(
            rules.find("udp dport 53").unwrap()
                < rules.find("ct state established ct mark !=").unwrap()
        );
        assert!(rules.contains("224.0.0.0/3"));
        assert!(rules.contains("ct mark set"));
        assert!(rules.contains("tproxy to :15260"));
    }

    #[test]
    fn cleanup_targets_only_owned_objects() {
        let rules = NetworkRules::new(config());
        let script = rules.nft_script();
        assert_eq!(script.matches("table inet canis_minor").count(), 1);
        assert!(!script.contains("flush ruleset"));
    }
}

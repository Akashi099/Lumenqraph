//! URL validation to prevent SSRF attacks in webhook subscriptions.

use std::net::IpAddr;
use url::Url;

pub fn validate_webhook_url(url: &str) -> Result<(), String> {
    let parsed = Url::parse(url).map_err(|e| format!("invalid URL: {}", e))?;

    match parsed.scheme() {
        "http" | "https" => {}
        _ => return Err("url scheme must be http or https".to_string()),
    }

    if let Some(host) = parsed.host_str() {
        if host.is_empty() {
            return Err("url must have a host".to_string());
        }

        if is_internal_address(host) {
            return Err(
                "url points to an internal/reserved address (loopback, link-local, private, or multicast)"
                    .to_string(),
            );
        }
    } else {
        return Err("url must have a host".to_string());
    }

    Ok(())
}

fn is_internal_address(host: &str) -> bool {
    if is_localhost(host) {
        return true;
    }

    if let Ok(ip) = host.parse::<IpAddr>() {
        return ip.is_loopback()
            || ip.is_private()
            || ip.is_link_local()
            || ip.is_multicast()
            || (match ip {
                IpAddr::V4(v4) => v4.is_reserved() || v4.is_documentation(),
                IpAddr::V6(v6) => v6.is_documentation(),
            });
    }

    false
}

fn is_localhost(host: &str) -> bool {
    matches!(
        host.to_lowercase().as_str(),
        "localhost" | "127.0.0.1" | "::1" | "[::1]"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_loopback_ips() {
        assert!(validate_webhook_url("http://127.0.0.1/hook").is_err());
        assert!(validate_webhook_url("http://127.0.0.2/hook").is_err());
        assert!(validate_webhook_url("http://[::1]/hook").is_err());
    }

    #[test]
    fn rejects_localhost_hostname() {
        assert!(validate_webhook_url("http://localhost/hook").is_err());
        assert!(validate_webhook_url("http://LOCALHOST/hook").is_err());
    }

    #[test]
    fn rejects_private_ips() {
        assert!(validate_webhook_url("http://10.0.0.1/hook").is_err());
        assert!(validate_webhook_url("http://172.16.0.1/hook").is_err());
        assert!(validate_webhook_url("http://192.168.1.1/hook").is_err());
        assert!(validate_webhook_url("http://[fc00::1]/hook").is_err());
    }

    #[test]
    fn rejects_link_local_ips() {
        assert!(validate_webhook_url("http://169.254.0.1/hook").is_err());
        assert!(validate_webhook_url("http://[fe80::1]/hook").is_err());
    }

    #[test]
    fn rejects_multicast_ips() {
        assert!(validate_webhook_url("http://224.0.0.1/hook").is_err());
        assert!(validate_webhook_url("http://[ff00::1]/hook").is_err());
    }

    #[test]
    fn rejects_aws_metadata_endpoint() {
        assert!(validate_webhook_url("http://169.254.169.254/latest/meta-data/").is_err());
    }

    #[test]
    fn rejects_kubernetes_metadata_endpoint() {
        assert!(validate_webhook_url("http://10.0.0.1:10250/api/v1/nodes").is_err());
    }

    #[test]
    fn accepts_public_urls() {
        assert!(validate_webhook_url("https://example.com/webhook").is_ok());
        assert!(validate_webhook_url("https://api.example.com:8080/hook").is_ok());
        assert!(validate_webhook_url("http://8.8.8.8/webhook").is_ok());
    }

    #[test]
    fn rejects_invalid_scheme() {
        assert!(validate_webhook_url("ftp://example.com/hook").is_err());
    }

    #[test]
    fn rejects_no_host() {
        assert!(validate_webhook_url("http:///hook").is_err());
    }
}

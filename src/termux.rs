/// Ensure $PREFIX/etc/resolv.conf exists on Termux.
///
/// Termux-native PHP reads $PREFIX/etc/resolv.conf. Static phm-managed PHP (musl +
/// c-ares) reads /etc/resolv.conf which is read-only on Android — handled by proot
/// wrapping in multishell and shim instead.
pub fn ensure_dns() {
    let Some(resolv_path) = resolv_conf_path() else {
        return;
    };

    if resolv_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&resolv_path) {
            if content.contains("nameserver") {
                return;
            }
        }
    }

    let content = build_resolv_conf();
    let _ = std::fs::write(&resolv_path, content);
}

pub fn is_termux() -> bool {
    std::env::var("PREFIX")
        .map(|p| p.contains("com.termux"))
        .unwrap_or(false)
}

pub fn resolv_conf_path() -> Option<std::path::PathBuf> {
    let prefix = std::env::var("PREFIX").ok()?;
    if !prefix.contains("com.termux") {
        return None;
    }
    Some(std::path::PathBuf::from(prefix).join("etc/resolv.conf"))
}

/// Find the proot binary on Termux.
pub fn proot_bin() -> Option<std::path::PathBuf> {
    if !is_termux() {
        return None;
    }
    if let Ok(prefix) = std::env::var("PREFIX") {
        let p = std::path::PathBuf::from(prefix).join("bin/proot");
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(output) = std::process::Command::new("which").arg("proot").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(std::path::PathBuf::from(path));
            }
        }
    }
    None
}

/// If proot DNS wrapping is needed, return `(proot_bin, resolv_conf_path)`.
///
/// Returns `None` when: not on Termux, /etc/resolv.conf is already readable, or proot is absent.
pub fn proot_wrap_args() -> Option<(std::path::PathBuf, std::path::PathBuf)> {
    if !is_termux() {
        return None;
    }
    let etc_resolv_ok = std::fs::read_to_string("/etc/resolv.conf")
        .map(|c| c.contains("nameserver"))
        .unwrap_or(false);
    if etc_resolv_ok {
        return None;
    }
    proot_bin().zip(resolv_conf_path())
}

fn build_resolv_conf() -> String {
    let mut lines = String::new();

    // Try to read Android's active DNS servers from system properties
    for prop in ["net.dns1", "net.dns2", "net.dns3", "net.dns4"] {
        if let Ok(output) = std::process::Command::new("getprop").arg(prop).output() {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !s.is_empty() && s != "0.0.0.0" {
                    lines.push_str(&format!("nameserver {}\n", s));
                }
            }
        }
    }

    if lines.is_empty() {
        lines.push_str("nameserver 8.8.8.8\n");
        lines.push_str("nameserver 8.8.4.4\n");
    }

    lines
}

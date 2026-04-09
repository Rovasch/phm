use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhpVersion {
    pub major: u8,
    pub minor: u8,
}

impl PhpVersion {
    pub fn new(major: u8, minor: u8) -> Self {
        Self { major, minor }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() < 2 {
            return None;
        }
        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        Some(Self { major, minor })
    }

    /// Parse a composer version constraint and extract the target version.
    /// Returns the minimum version that satisfies the constraint.
    pub fn from_constraint(constraint: &str) -> Option<Self> {
        let constraint = constraint.trim();

        // Handle OR constraints: "^7.4 || ^8.0" or "^7.4|^8.0"
        if constraint.contains("||") || constraint.contains('|') {
            let parts: Vec<&str> = if constraint.contains("||") {
                constraint.split("||").collect()
            } else {
                constraint.split('|').collect()
            };

            // Parse all alternatives, return the one from the highest group
            // (prefer newest major version)
            let mut versions: Vec<PhpVersion> = parts
                .iter()
                .filter_map(|p| Self::from_single_constraint(p))
                .collect();
            versions.sort();
            // Return lowest from highest major group
            return versions.last().cloned();
        }

        // Handle AND constraints: ">=8.1 <9.0" — take the lower bound
        if constraint.contains(' ') && !constraint.contains("||") {
            let parts: Vec<&str> = constraint.split_whitespace().collect();
            for part in &parts {
                let trimmed = part.trim_start_matches(">=").trim_start_matches('>');
                if part.starts_with(">=") || part.starts_with('>') {
                    if let Some(v) = Self::parse(trimmed) {
                        return Some(v);
                    }
                }
            }
            // Fallback: parse first part
            return Self::from_single_constraint(parts[0]);
        }

        Self::from_single_constraint(constraint)
    }

    fn from_single_constraint(s: &str) -> Option<Self> {
        let s = s.trim();

        // Strip constraint operators
        let version_str = s
            .trim_start_matches(">=")
            .trim_start_matches("<=")
            .trim_start_matches('>')
            .trim_start_matches('<')
            .trim_start_matches('^')
            .trim_start_matches('~')
            .trim();

        // Handle wildcard: "8.2.*" -> "8.2"
        let version_str = version_str.trim_end_matches(".*").trim_end_matches(".*");

        Self::parse(version_str)
    }

    /// Find the lowest installed version that satisfies the constraint.
    pub fn resolve(constraint: &str, installed: &[PhpVersion]) -> Option<PhpVersion> {
        let target = Self::from_constraint(constraint)?;

        let mut candidates: Vec<&PhpVersion> = installed
            .iter()
            .filter(|v| **v >= target)
            .collect();
        candidates.sort();
        candidates.first().cloned().cloned()
    }
}

impl fmt::Display for PhpVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        assert_eq!(PhpVersion::parse("8.2"), Some(PhpVersion::new(8, 2)));
        assert_eq!(PhpVersion::parse("7.4"), Some(PhpVersion::new(7, 4)));
        assert_eq!(PhpVersion::parse("8.2.30"), Some(PhpVersion::new(8, 2)));
    }

    #[test]
    fn test_constraints() {
        assert_eq!(
            PhpVersion::from_constraint(">=8.2"),
            Some(PhpVersion::new(8, 2))
        );
        assert_eq!(
            PhpVersion::from_constraint("^8.2"),
            Some(PhpVersion::new(8, 2))
        );
        assert_eq!(
            PhpVersion::from_constraint("~8.2"),
            Some(PhpVersion::new(8, 2))
        );
        assert_eq!(
            PhpVersion::from_constraint("8.2.*"),
            Some(PhpVersion::new(8, 2))
        );
        assert_eq!(
            PhpVersion::from_constraint("~8.2.0"),
            Some(PhpVersion::new(8, 2))
        );
    }

    #[test]
    fn test_or_constraints() {
        // Should pick highest group
        assert_eq!(
            PhpVersion::from_constraint("^7.4 || ^8.0"),
            Some(PhpVersion::new(8, 0))
        );
        assert_eq!(
            PhpVersion::from_constraint("^7.4|^8.0"),
            Some(PhpVersion::new(8, 0))
        );
    }

    #[test]
    fn test_resolve() {
        let installed = vec![
            PhpVersion::new(7, 4),
            PhpVersion::new(8, 1),
            PhpVersion::new(8, 2),
            PhpVersion::new(8, 4),
            PhpVersion::new(8, 5),
        ];

        // >=8.2 should resolve to 8.2 (lowest matching)
        assert_eq!(
            PhpVersion::resolve(">=8.2", &installed),
            Some(PhpVersion::new(8, 2))
        );

        // ^8.1 should resolve to 8.1
        assert_eq!(
            PhpVersion::resolve("^8.1", &installed),
            Some(PhpVersion::new(8, 1))
        );

        // >=9.0 should resolve to None
        assert_eq!(PhpVersion::resolve(">=9.0", &installed), None);
    }
}

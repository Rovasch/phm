use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhpVersion {
    pub major: u8,
    pub minor: u8,
}

impl PhpVersion {
    #[cfg(test)]
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
}

impl fmt::Display for PhpVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// A version constraint with min and optional exclusive upper bound.
/// Properly models composer semantics: `8.4.*` only matches 8.4,
/// `^8.4` matches 8.4–8.x, `>=8.4` is open-ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionConstraint {
    pub min: PhpVersion,
    pub max: Option<PhpVersion>, // exclusive upper bound
}

impl VersionConstraint {
    /// Constraint that matches exactly one minor version.
    pub fn exact(version: PhpVersion) -> Self {
        Self {
            min: version,
            max: Some(PhpVersion {
                major: version.major,
                minor: version.minor + 1,
            }),
        }
    }

    /// Parse a composer-style version constraint string.
    pub fn from_constraint(constraint: &str) -> Option<Self> {
        let constraint = constraint.trim();

        // Handle OR constraints: "^7.4 || ^8.0" or "^7.4|^8.0"
        if constraint.contains("||") || constraint.contains('|') {
            let parts: Vec<&str> = if constraint.contains("||") {
                constraint.split("||").collect()
            } else {
                constraint.split('|').collect()
            };
            let mut constraints: Vec<VersionConstraint> = parts
                .iter()
                .filter_map(|p| Self::from_single_constraint(p))
                .collect();
            constraints.sort_by_key(|c| c.min);
            // Prefer the highest minimum (newest)
            return constraints.into_iter().last();
        }

        // Handle AND constraints: ">=8.1 <9.0"
        if constraint.contains(' ') {
            let parts: Vec<&str> = constraint.split_whitespace().collect();
            let mut min = None;
            let mut max = None;
            for part in &parts {
                if part.starts_with(">=") {
                    min = PhpVersion::parse(part.trim_start_matches(">="));
                } else if part.starts_with('>') {
                    if let Some(v) = PhpVersion::parse(part.trim_start_matches('>')) {
                        min = Some(PhpVersion {
                            major: v.major,
                            minor: v.minor + 1,
                        });
                    }
                } else if part.starts_with("<=") {
                    if let Some(v) = PhpVersion::parse(part.trim_start_matches("<=")) {
                        max = Some(PhpVersion {
                            major: v.major,
                            minor: v.minor + 1,
                        });
                    }
                } else if part.starts_with('<') {
                    max = PhpVersion::parse(part.trim_start_matches('<'));
                }
            }
            if let Some(min_v) = min {
                return Some(Self { min: min_v, max });
            }
            return Self::from_single_constraint(parts[0]);
        }

        Self::from_single_constraint(constraint)
    }

    fn from_single_constraint(s: &str) -> Option<Self> {
        let s = s.trim();

        // Wildcard: "8.4.*" → exact 8.4
        if s.ends_with(".*") {
            let v = PhpVersion::parse(s.trim_end_matches(".*"))?;
            return Some(Self::exact(v));
        }

        // Tilde: ~X.Y.Z → exact minor, ~X.Y → same major
        if s.starts_with('~') {
            let version_str = s.trim_start_matches('~').trim();
            let parts: Vec<&str> = version_str.split('.').collect();
            let v = PhpVersion::parse(version_str)?;
            if parts.len() >= 3 {
                // ~8.4.0 → >=8.4.0 <8.5.0 → exact minor
                return Some(Self::exact(v));
            }
            // ~8.4 → >=8.4.0 <9.0.0
            return Some(Self {
                min: v,
                max: Some(PhpVersion {
                    major: v.major + 1,
                    minor: 0,
                }),
            });
        }

        // Caret: ^X.Y → same major (for X > 0)
        if s.starts_with('^') {
            let v = PhpVersion::parse(s.trim_start_matches('^').trim())?;
            if v.major == 0 {
                // ^0.3 → >=0.3.0 <0.4.0
                return Some(Self {
                    min: v,
                    max: Some(PhpVersion {
                        major: 0,
                        minor: v.minor + 1,
                    }),
                });
            }
            // ^8.4 → >=8.4.0 <9.0.0
            return Some(Self {
                min: v,
                max: Some(PhpVersion {
                    major: v.major + 1,
                    minor: 0,
                }),
            });
        }

        // >= open-ended
        if s.starts_with(">=") {
            let v = PhpVersion::parse(s.trim_start_matches(">=").trim())?;
            return Some(Self { min: v, max: None });
        }

        // > exclusive lower bound
        if s.starts_with('>') {
            let v = PhpVersion::parse(s.trim_start_matches('>').trim())?;
            return Some(Self {
                min: PhpVersion {
                    major: v.major,
                    minor: v.minor + 1,
                },
                max: None,
            });
        }

        // Plain version: exact match
        let v = PhpVersion::parse(s)?;
        Some(Self::exact(v))
    }

    /// Check if a version satisfies this constraint.
    pub fn satisfies(&self, version: PhpVersion) -> bool {
        version >= self.min && self.max.is_none_or(|max| version < max)
    }

    /// Find the lowest installed version that satisfies this constraint.
    pub fn resolve(&self, installed: &[PhpVersion]) -> Option<PhpVersion> {
        installed
            .iter()
            .filter(|v| self.satisfies(**v))
            .min()
            .copied()
    }

    /// The minimum version of this constraint (used for install suggestions).
    pub fn target(&self) -> PhpVersion {
        self.min
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
        // >=8.2 → open-ended
        assert_eq!(
            VersionConstraint::from_constraint(">=8.2"),
            Some(VersionConstraint {
                min: PhpVersion::new(8, 2),
                max: None
            })
        );
        // ^8.2 → 8.2–8.x (max 9.0)
        assert_eq!(
            VersionConstraint::from_constraint("^8.2"),
            Some(VersionConstraint {
                min: PhpVersion::new(8, 2),
                max: Some(PhpVersion::new(9, 0))
            })
        );
        // ~8.2 → 8.2–8.x (max 9.0)
        assert_eq!(
            VersionConstraint::from_constraint("~8.2"),
            Some(VersionConstraint {
                min: PhpVersion::new(8, 2),
                max: Some(PhpVersion::new(9, 0))
            })
        );
        // 8.2.* → exact 8.2
        assert_eq!(
            VersionConstraint::from_constraint("8.2.*"),
            Some(VersionConstraint::exact(PhpVersion::new(8, 2)))
        );
        // ~8.2.0 → exact 8.2
        assert_eq!(
            VersionConstraint::from_constraint("~8.2.0"),
            Some(VersionConstraint::exact(PhpVersion::new(8, 2)))
        );
    }

    #[test]
    fn test_or_constraints() {
        // Should pick highest group
        assert_eq!(
            VersionConstraint::from_constraint("^7.4 || ^8.0").map(|c| c.min),
            Some(PhpVersion::new(8, 0))
        );
        assert_eq!(
            VersionConstraint::from_constraint("^7.4|^8.0").map(|c| c.min),
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

        // >=8.2 → 8.2 (lowest matching, open-ended)
        assert_eq!(
            VersionConstraint::from_constraint(">=8.2")
                .unwrap()
                .resolve(&installed),
            Some(PhpVersion::new(8, 2))
        );

        // ^8.1 → 8.1
        assert_eq!(
            VersionConstraint::from_constraint("^8.1")
                .unwrap()
                .resolve(&installed),
            Some(PhpVersion::new(8, 1))
        );

        // >=9.0 → None
        assert_eq!(
            VersionConstraint::from_constraint(">=9.0")
                .unwrap()
                .resolve(&installed),
            None
        );

        // 8.4.* → exactly 8.4
        assert_eq!(
            VersionConstraint::from_constraint("8.4.*")
                .unwrap()
                .resolve(&installed),
            Some(PhpVersion::new(8, 4))
        );
    }

    #[test]
    fn test_wildcard_no_fallback() {
        // Bug: 8.4.* was resolving to 8.5 when 8.4 not installed
        let installed = vec![PhpVersion::new(8, 3), PhpVersion::new(8, 5)];
        assert_eq!(
            VersionConstraint::from_constraint("8.4.*")
                .unwrap()
                .resolve(&installed),
            None
        );
    }

    #[test]
    fn test_satisfies() {
        let exact = VersionConstraint::exact(PhpVersion::new(8, 4));
        assert!(exact.satisfies(PhpVersion::new(8, 4)));
        assert!(!exact.satisfies(PhpVersion::new(8, 5)));
        assert!(!exact.satisfies(PhpVersion::new(8, 3)));

        let caret = VersionConstraint::from_constraint("^8.2").unwrap();
        assert!(caret.satisfies(PhpVersion::new(8, 2)));
        assert!(caret.satisfies(PhpVersion::new(8, 5)));
        assert!(!caret.satisfies(PhpVersion::new(9, 0)));
        assert!(!caret.satisfies(PhpVersion::new(8, 1)));

        let open = VersionConstraint::from_constraint(">=8.2").unwrap();
        assert!(open.satisfies(PhpVersion::new(8, 2)));
        assert!(open.satisfies(PhpVersion::new(9, 0)));
        assert!(!open.satisfies(PhpVersion::new(8, 1)));
    }
}

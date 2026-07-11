/// Runtime disclosure shown to the operator. These rules are deliberately not
/// configurable: monitoring must never read filesystem files or raw user data.
#[derive(Debug, Clone)]
pub struct SecurityPosture {
    pub outbound_notifications: bool,
    pub legacy_external_collectors: bool,
}

impl SecurityPosture {
    pub fn new(outbound_notifications: bool) -> Self {
        Self {
            outbound_notifications,
            legacy_external_collectors: true,
        }
    }

    pub fn disclosure(&self) -> String {
        let network = if self.outbound_notifications {
            "Discord ON"
        } else {
            "network OFF"
        };
        let collectors = if self.legacy_external_collectors {
            "legacy tools ON"
        } else {
            "native collectors"
        };
        format!("metadata only · content access DENIED · {network} · {collectors}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disclosure_never_implies_content_access() {
        let text = SecurityPosture::new(false).disclosure();
        assert!(text.contains("content access DENIED"));
        assert!(text.contains("network OFF"));
    }
}

/// Runtime disclosure shown to the operator. These rules are deliberately not
/// configurable: monitoring must never read filesystem files or raw user data.
#[derive(Debug, Clone)]
pub struct SecurityPosture {
    pub outbound_notifications: bool,
    pub legacy_external_collectors: bool,
    pub privileged_broker: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorCapability {
    StorageMetadata,
    KernelCounters,
    HealthMetadata,
    OutboundNotification,
    FilesystemContent,
    RawSectors,
    ArbitraryPrivilegedCommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyDecision {
    Allowed,
    Denied,
}

impl SecurityPosture {
    pub fn new(outbound_notifications: bool) -> Self {
        Self {
            outbound_notifications,
            legacy_external_collectors: true,
            privileged_broker: false,
        }
    }

    pub const fn decision(&self, capability: MonitorCapability) -> PolicyDecision {
        match capability {
            MonitorCapability::StorageMetadata
            | MonitorCapability::KernelCounters
            | MonitorCapability::HealthMetadata => PolicyDecision::Allowed,
            MonitorCapability::OutboundNotification if self.outbound_notifications => {
                PolicyDecision::Allowed
            }
            MonitorCapability::OutboundNotification
            | MonitorCapability::FilesystemContent
            | MonitorCapability::RawSectors
            | MonitorCapability::ArbitraryPrivilegedCommand => PolicyDecision::Denied,
        }
    }

    pub fn disclosure(&self) -> String {
        debug_assert_eq!(
            self.decision(MonitorCapability::StorageMetadata),
            PolicyDecision::Allowed
        );
        debug_assert_eq!(
            self.decision(MonitorCapability::KernelCounters),
            PolicyDecision::Allowed
        );
        debug_assert_eq!(
            self.decision(MonitorCapability::HealthMetadata),
            PolicyDecision::Allowed
        );
        debug_assert_eq!(
            self.decision(MonitorCapability::RawSectors),
            PolicyDecision::Denied
        );
        debug_assert_eq!(
            self.decision(MonitorCapability::ArbitraryPrivilegedCommand),
            PolicyDecision::Denied
        );

        let network =
            if self.decision(MonitorCapability::OutboundNotification) == PolicyDecision::Allowed {
                "Discord ON"
            } else {
                "network OFF"
            };
        let collectors = if self.legacy_external_collectors {
            "legacy tools ON"
        } else {
            "native collectors"
        };
        let privilege = if self.privileged_broker {
            "privileged broker ON"
        } else {
            "privileged broker OFF"
        };
        let content =
            if self.decision(MonitorCapability::FilesystemContent) == PolicyDecision::Denied {
                "content access DENIED"
            } else {
                "content access allowed"
            };
        format!("metadata only · {content} · {network} · {collectors} · {privilege}")
    }

    pub fn compact_disclosure(&self) -> String {
        let network =
            if self.decision(MonitorCapability::OutboundNotification) == PolicyDecision::Allowed {
                "net ON"
            } else {
                "net OFF"
            };
        let collectors = if self.legacy_external_collectors {
            "legacy ON"
        } else {
            "native"
        };
        let broker = if self.privileged_broker {
            "broker ON"
        } else {
            "broker OFF"
        };
        format!("meta · content DENIED · {network} · {collectors} · {broker}")
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
        assert!(text.contains("privileged broker OFF"));
    }

    #[test]
    fn monitoring_metadata_is_allowed_but_user_content_and_raw_commands_are_denied() {
        let posture = SecurityPosture::new(false);

        assert_eq!(
            posture.decision(MonitorCapability::StorageMetadata),
            PolicyDecision::Allowed
        );
        assert_eq!(
            posture.decision(MonitorCapability::KernelCounters),
            PolicyDecision::Allowed
        );
        assert_eq!(
            posture.decision(MonitorCapability::HealthMetadata),
            PolicyDecision::Allowed
        );
        assert_eq!(
            posture.decision(MonitorCapability::FilesystemContent),
            PolicyDecision::Denied
        );
        assert_eq!(
            posture.decision(MonitorCapability::RawSectors),
            PolicyDecision::Denied
        );
        assert_eq!(
            posture.decision(MonitorCapability::ArbitraryPrivilegedCommand),
            PolicyDecision::Denied
        );
    }

    #[test]
    fn outbound_notification_requires_explicit_configuration() {
        assert_eq!(
            SecurityPosture::new(false).decision(MonitorCapability::OutboundNotification),
            PolicyDecision::Denied
        );
        assert_eq!(
            SecurityPosture::new(true).decision(MonitorCapability::OutboundNotification),
            PolicyDecision::Allowed
        );
    }

    #[test]
    fn compact_disclosure_keeps_every_security_dimension_visible() {
        let text = SecurityPosture::new(false).compact_disclosure();
        assert!(text.contains("content DENIED"));
        assert!(text.contains("net OFF"));
        assert!(text.contains("legacy ON"));
        assert!(text.contains("broker OFF"));
    }
}

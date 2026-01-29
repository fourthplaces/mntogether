use intelligent_crawler::{CrawlerCommand, CrawlerEvent};
use seesaw::Machine;
use tracing::info;
use uuid::Uuid;

/// Resource aggregate state (built from events)
#[derive(Debug, Clone)]
struct ResourceState {
    resource_id: Uuid,
    discovery_status: DiscoveryStatus,
    discovery_version: i32,
    pages_discovered: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiscoveryStatus {
    Pending,
    Discovering,
    Completed,
    Failed,
}

impl ResourceState {
    fn new(resource_id: Uuid) -> Self {
        Self {
            resource_id,
            discovery_status: DiscoveryStatus::Pending,
            discovery_version: 0,
            pages_discovered: 0,
        }
    }

    /// Apply event to update state
    fn apply(&mut self, event: &CrawlerEvent) {
        match event {
            CrawlerEvent::ResourceSubmitted { .. } => {
                // Initial state already set
            }
            CrawlerEvent::DiscoveryStarted { .. } => {
                self.discovery_status = DiscoveryStatus::Discovering;
            }
            CrawlerEvent::PageDiscovered { .. } => {
                self.pages_discovered += 1;
            }
            CrawlerEvent::DiscoveryCompleted { resource_id, .. } => {
                if *resource_id == self.resource_id {
                    self.discovery_status = DiscoveryStatus::Completed;
                    self.discovery_version += 1;
                }
            }
            CrawlerEvent::DiscoveryFailed { resource_id, .. } => {
                if *resource_id == self.resource_id {
                    self.discovery_status = DiscoveryStatus::Failed;
                }
            }
            _ => {}
        }
    }
}

/// Resource discovery state machine
/// Manages resource discovery lifecycle
pub struct ResourceDiscoveryMachine {
    state: Option<ResourceState>,
}

impl ResourceDiscoveryMachine {
    pub fn new() -> Self {
        Self { state: None }
    }
}

impl Machine for ResourceDiscoveryMachine {
    type Event = CrawlerEvent;
    type Command = CrawlerCommand;

    fn decide(&mut self, event: &CrawlerEvent) -> Option<CrawlerCommand> {
        // Initialize or update state
        match event {
            CrawlerEvent::ResourceSubmitted { resource_id, url, submitted_by: _ } => {
                info!(resource_id = %resource_id, url = %url, "Resource submitted");

                // Initialize state for new resource
                self.state = Some(ResourceState::new(*resource_id));

                // Start discovery
                return Some(CrawlerCommand::DiscoverResource {
                    resource_id: *resource_id,
                    max_depth: 2,
                    same_domain_only: true,
                });
            }
            _ => {
                // Update existing state
                if let Some(ref mut state) = self.state {
                    state.apply(event);
                }
            }
        }

        // Decide commands based on state
        if let Some(ref state) = self.state {
            match state.discovery_status {
                DiscoveryStatus::Completed => {
                    // Could schedule rediscovery here in the future
                    None
                }
                DiscoveryStatus::Failed => {
                    // Could retry or notify here
                    None
                }
                _ => None,
            }
        } else {
            None
        }
    }
}

impl Default for ResourceDiscoveryMachine {
    fn default() -> Self {
        Self::new()
    }
}

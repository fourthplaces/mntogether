use intelligent_crawler::{AggregateKey, CrawlerCommand, CrawlerEvent};
use seesaw_core::Machine;
use std::collections::HashMap;
use uuid::Uuid;

use super::machines::{PageLifecycleMachine, ResourceDiscoveryMachine};

/// Crawler coordinator that routes events to the appropriate state machines
/// and collects commands from them
pub struct CrawlerCoordinator {
    resource_machines: HashMap<Uuid, ResourceDiscoveryMachine>,
    page_machines: HashMap<Uuid, PageLifecycleMachine>,
}

impl CrawlerCoordinator {
    pub fn new() -> Self {
        Self {
            resource_machines: HashMap::new(),
            page_machines: HashMap::new(),
        }
    }

    /// Process an event and collect commands from state machines
    pub fn process_event(&mut self, event: &CrawlerEvent) -> Vec<CrawlerCommand> {
        let mut commands = Vec::new();

        // Route event to the appropriate state machine based on aggregate
        match event.aggregate_key() {
            AggregateKey::Resource(resource_id) => {
                // Get or create resource machine
                let machine = self
                    .resource_machines
                    .entry(resource_id)
                    .or_insert_with(ResourceDiscoveryMachine::new);

                // Process event and collect commands
                if let Some(cmd) = machine.decide(event) {
                    commands.push(cmd);
                }
            }
            AggregateKey::Page(page_id) => {
                // Get or create page machine
                let machine = self
                    .page_machines
                    .entry(page_id)
                    .or_insert_with(PageLifecycleMachine::new);

                // Process event and collect commands
                if let Some(cmd) = machine.decide(event) {
                    commands.push(cmd);
                }
            }
            AggregateKey::Extraction(_) => {
                // No machine for extractions yet (they're terminal events)
            }
        }

        commands
    }

    /// Get statistics about the coordinator
    pub fn stats(&self) -> CoordinatorStats {
        CoordinatorStats {
            resource_machines: self.resource_machines.len(),
            page_machines: self.page_machines.len(),
        }
    }
}

impl Default for CrawlerCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct CoordinatorStats {
    pub resource_machines: usize,
    pub page_machines: usize,
}

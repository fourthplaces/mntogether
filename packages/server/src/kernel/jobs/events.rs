use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ErrorKind, Job};

/// Job lifecycle events.
///
/// These events represent facts about the job lifecycle, not commands.
/// They flow through the Seesaw event system for unified
/// tracing and consistent effect handling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobEvent {
    /// A job was scheduled (to be persisted to DB and potentially published to NATS).
    Scheduled { job: Job },

    /// A job is ready to execute (received from NATS or scheduler poll).
    Ready { job: Job },

    /// Job execution started.
    Started {
        job_id: Uuid,
        root_job_id: Option<Uuid>,
        job_type: String,
        worker_id: String,
        attempt: i32,
    },

    /// Job completed successfully.
    Succeeded {
        job_id: Uuid,
        root_job_id: Option<Uuid>,
        job_type: String,
        duration_ms: u64,
    },

    /// Job execution failed.
    Failed {
        job_id: Uuid,
        root_job_id: Option<Uuid>,
        job_type: String,
        error: String,
        error_kind: ErrorKind,
        attempt: i32,
        will_retry: bool,
    },

    /// Job moved to dead letter queue (exhausted retries or non-retryable error).
    DeadLettered {
        job_id: Uuid,
        root_job_id: Option<Uuid>,
        job_type: String,
        total_attempts: i32,
        final_error: String,
    },

    /// Job was cancelled.
    Cancelled {
        job_id: Uuid,
        job_type: String,
        reason: Option<String>,
    },

    /// A stale job's lease expired and was recovered by another worker.
    LeaseRecovered {
        job_id: Uuid,
        job_type: String,
        old_worker_id: Option<String>,
        new_worker_id: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_job() -> Job {
        Job::immediate(
            Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            "test_job",
        )
    }

    #[test]
    fn event_scheduled_serializes() {
        let event = JobEvent::Scheduled { job: sample_job() };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Scheduled"));
        assert!(json.contains("test_job"));
    }

    #[test]
    fn event_ready_serializes() {
        let event = JobEvent::Ready { job: sample_job() };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Ready"));
    }

    #[test]
    fn event_started_serializes() {
        let event = JobEvent::Started {
            job_id: Uuid::new_v4(),
            root_job_id: None,
            job_type: "test_job".to_string(),
            worker_id: "worker-1".to_string(),
            attempt: 1,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Started"));
        assert!(json.contains("test_job"));
        assert!(json.contains("worker-1"));
    }

    #[test]
    fn event_succeeded_serializes() {
        let event = JobEvent::Succeeded {
            job_id: Uuid::new_v4(),
            root_job_id: None,
            job_type: "test_job".to_string(),
            duration_ms: 1500,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Succeeded"));
        assert!(json.contains("1500"));
    }

    #[test]
    fn event_failed_serializes() {
        let event = JobEvent::Failed {
            job_id: Uuid::new_v4(),
            root_job_id: None,
            job_type: "test_job".to_string(),
            error: "Something went wrong".to_string(),
            error_kind: ErrorKind::Retryable,
            attempt: 1,
            will_retry: true,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Failed"));
        assert!(json.contains("Something went wrong"));
        assert!(json.contains("will_retry"));
    }

    #[test]
    fn event_dead_lettered_serializes() {
        let event = JobEvent::DeadLettered {
            job_id: Uuid::new_v4(),
            root_job_id: Some(Uuid::new_v4()),
            job_type: "test_job".to_string(),
            total_attempts: 3,
            final_error: "Max retries exceeded".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("DeadLettered"));
        assert!(json.contains("total_attempts"));
    }

    #[test]
    fn event_cancelled_serializes() {
        let event = JobEvent::Cancelled {
            job_id: Uuid::new_v4(),
            job_type: "test_job".to_string(),
            reason: Some("User requested".to_string()),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Cancelled"));
    }

    #[test]
    fn event_lease_recovered_serializes() {
        let event = JobEvent::LeaseRecovered {
            job_id: Uuid::new_v4(),
            job_type: "test_job".to_string(),
            old_worker_id: Some("worker-1".to_string()),
            new_worker_id: "worker-2".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("LeaseRecovered"));
    }

    #[test]
    fn events_roundtrip_serialize() {
        let events = vec![
            JobEvent::Scheduled { job: sample_job() },
            JobEvent::Ready { job: sample_job() },
            JobEvent::Started {
                job_id: Uuid::new_v4(),
                root_job_id: None,
                job_type: "test".to_string(),
                worker_id: "worker-1".to_string(),
                attempt: 1,
            },
            JobEvent::Succeeded {
                job_id: Uuid::new_v4(),
                root_job_id: None,
                job_type: "test".to_string(),
                duration_ms: 100,
            },
            JobEvent::Failed {
                job_id: Uuid::new_v4(),
                root_job_id: None,
                job_type: "test".to_string(),
                error: "err".to_string(),
                error_kind: ErrorKind::Retryable,
                attempt: 1,
                will_retry: false,
            },
            JobEvent::DeadLettered {
                job_id: Uuid::new_v4(),
                root_job_id: None,
                job_type: "test".to_string(),
                total_attempts: 3,
                final_error: "failed".to_string(),
            },
        ];

        for event in events {
            let json = serde_json::to_string(&event).unwrap();
            let _: JobEvent = serde_json::from_str(&json).unwrap();
        }
    }
}

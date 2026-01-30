# Authorization Policy

## Overview

This document describes the authorization architecture and policy for the server application, following the seesaw-rs event-driven pattern.

## Architecture

The application uses an **event-driven authorization model** where authorization checks can occur at multiple layers:

1. **GraphQL Edge Layer** (Primary) - Mutations and queries validate permissions before dispatching commands
2. **Effect Layer** (Optional) - Effects may perform authorization checks when handling commands
3. **Machine Layer** (None) - State machines contain no authorization logic (pure decision logic)

## Current Authorization Pattern

### Edge Layer Authorization (Recommended)

Authorization checks **should** occur at the GraphQL edge layer before dispatching commands to the event system. This provides:

- Early rejection of unauthorized requests
- Consistent error responses to clients
- Separation of authorization from business logic
- Single source of truth for permissions

### Effect Layer Authorization (Selective)

Some effects perform authorization checks when they execute commands. This pattern is used when:

- The command requires context-dependent authorization (e.g., checking resource ownership)
- Authorization logic is tightly coupled with the effect's operation
- Multiple entry points to the same effect require consistent authorization

**Example:** `ScraperEffect` checks `AdminCapability::TriggerScraping` because scraping operations are sensitive and require explicit admin permission.

```rust
// Effect-level authorization (ScraperEffect)
if let Err(auth_err) = Actor::new(requested_by, is_admin)
    .can(AdminCapability::TriggerScraping)
    .check(ctx.deps())
    .await
{
    return Ok(ListingEvent::AuthorizationDenied {
        requested_by,
        reason: auth_err.to_string(),
    });
}
```

## Current Status

### Effects WITH Authorization Checks

- **ScraperEffect**: Checks `AdminCapability::TriggerScraping`

### Effects WITHOUT Authorization Checks

- **AIEffect**: No authorization check (assumes pre-authorized at edge)
- **SyncEffect**: No authorization check (assumes pre-authorized at edge)
- **SearchEffect**: No authorization check (assumes pre-authorized at edge)
- **ListingEffect**: No authorization check (assumes pre-authorized at edge)

## Recommendation

**Primary Policy:** Perform authorization at the GraphQL edge layer before dispatching commands.

**Secondary Policy:** Add effect-level authorization only when:
1. The operation is particularly sensitive (e.g., data deletion, system configuration)
2. Multiple entry points exist and edge-level auth cannot cover all cases
3. Authorization depends on runtime context not available at the edge

## Authorization Flow

```
┌─────────────┐
│   Client    │
└──────┬──────┘
       │ GraphQL Request
       ▼
┌─────────────────────┐
│   Edge (Mutation)   │ ◄─── PRIMARY: Authorize here
│  - Check session    │
│  - Check capability │
└──────┬──────────────┘
       │ Dispatch Command
       ▼
┌─────────────────────┐
│      Machine        │
│  (Pure Logic)       │
└──────┬──────────────┘
       │ Emit Command
       ▼
┌─────────────────────┐
│      Effect         │ ◄─── OPTIONAL: Double-check for sensitive ops
│  - Execute IO       │
│  - May re-authorize │
└──────┬──────────────┘
       │ Return Event
       ▼
┌─────────────────────┐
│   Event Store       │
└─────────────────────┘
```

## Authorization Components

### Actor

Represents a user or system performing an action.

```rust
Actor::new(member_id, is_admin)
```

### Capabilities

Define what actions an actor can perform:

- `AdminCapability::TriggerScraping` - Manually trigger website scraping
- `AdminCapability::ManageUsers` - Create, update, delete users
- `AdminCapability::ModerateContent` - Approve, reject, edit listings
- (Additional capabilities defined in `src/common/auth/capability.rs`)

### Authorization Check

```rust
Actor::new(member_id, is_admin)
    .can(SomeCapability)
    .check(deps)
    .await?
```

Returns:
- `Ok(())` if authorized
- `Err(AuthError)` if not authorized

## Authorization Events

When authorization fails in an effect, return an authorization denied event:

```rust
ListingEvent::AuthorizationDenied {
    requested_by: member_id,
    reason: "Insufficient permissions".to_string(),
}
```

This allows the machine to handle unauthorized attempts gracefully and emit appropriate error responses.

## Future Improvements

1. **Consistent Edge Authorization**: Ensure all GraphQL mutations check authorization before dispatching commands
2. **Remove Redundant Effect Auth**: Remove effect-level authorization for operations already protected at the edge
3. **Capability Registry**: Document all capabilities and their required permissions in a central location
4. **Audit Trail**: Log all authorization checks (both successes and failures) for security auditing
5. **Resource-Level Permissions**: Add support for checking ownership/membership before allowing operations on specific resources

## Related Files

- `src/common/auth/` - Authorization builder, capabilities, and errors
- `src/domains/listings/effects/scraper.rs` - Example effect-level authorization
- `src/domains/listings/events/mod.rs` - Authorization denied event definitions
- `src/domains/listings/edges/mutation.rs` - GraphQL mutations (edge layer)

# User-Submitted Needs

## Overview

Volunteers can submit needs they encounter, not just see scraped ones. This allows the app to crowdsource need discovery.

## How It Works

### 1. Volunteer Submits Need

Via Expo app:
```graphql
mutation SubmitNeed($input: SubmitNeedInput!) {
  submitNeed(input: $input, volunteerId: $volunteerId) {
    id
    title
    status  # Will be PENDING_APPROVAL
  }
}
```

Input:
- Organization name (or "Community Member" if individual)
- Title
- Description
- Contact info (optional)
- Urgency (optional)
- Location (optional)

### 2. Goes to Pending Approval

**All user-submitted needs start as `pending_approval`** - same as AI-extracted needs.

**Why?**
- Quality control (prevent spam)
- Verify accuracy (is this a real need?)
- Format consistency (admin can clean up descriptions)

### 3. Admin Reviews in Same Queue

Admin sees in approval queue:
- 🌐 **AI-extracted** (from website scrape)
- 👤 **User-submitted** (from volunteer in app)

Admin can:
- ✅ Approve → Status: "active" → Visible to all volunteers
- ✏️ Edit + Approve → Fix description, add context
- ❌ Reject → Status: "rejected" → Hidden forever

### 4. Approved Needs Visible to All

Once approved, user-submitted needs appear in the same feed as scraped needs.

## Privacy & Geolocation

### IP Address Storage

We store IP addresses for:
1. **Geolocation** - Match volunteers to nearby needs
2. **Spam prevention** - Detect abuse patterns
3. **Analytics** - Understand where needs are being reported

### Geolocation Fields

**For Volunteers:**
- `ip_address` - Stored as PostgreSQL INET type
- `city`, `state`, `country` - Extracted from IP
- `latitude`, `longitude` - Approximate location (city-level, not exact)

**For Needs:**
- `submitted_from_ip` - IP address of submitter
- `location` - Text description (e.g., "North Minneapolis")

### Geolocation Service

We'll use a service like:
- **ipapi.co** (free tier: 1,000 requests/day)
- **ipinfo.io** (free tier: 50,000 requests/month)
- **ip-api.com** (free for non-commercial)

Example:
```rust
async fn geolocate_ip(ip: IpAddr) -> Result<GeoLocation> {
    let url = format!("http://ip-api.com/json/{}", ip);
    let response: IpApiResponse = reqwest::get(&url).await?.json().await?;

    Ok(GeoLocation {
        city: response.city,
        state: response.region_name,
        country: response.country_code,
        latitude: response.lat,
        longitude: response.lon,
    })
}
```

## Database Schema

```sql
-- organization_needs table
CREATE TABLE organization_needs (
    -- ... existing fields ...

    submission_type TEXT DEFAULT 'scraped',  -- 'scraped' | 'user_submitted'
    submitted_by_volunteer_id UUID REFERENCES volunteers(id),
    submitted_from_ip INET,  -- For geolocation + spam prevention
    location TEXT,  -- User-provided location ("North Minneapolis")

    -- ... rest of schema ...
);

-- volunteers table
CREATE TABLE volunteers (
    -- ... existing fields ...

    ip_address INET,  -- For geolocation
    city TEXT,  -- "Minneapolis"
    state TEXT,  -- "Minnesota"
    country TEXT DEFAULT 'US',
    latitude NUMERIC(10, 8),  -- Approximate (city-level)
    longitude NUMERIC(11, 8),

    -- ... rest of schema ...
);
```

## UI Examples

### Expo App - Submit Need Screen

```typescript
<Form>
  <Input
    label="Organization or Community Group"
    placeholder="Community Center, Church, Nonprofit..."
    value={organizationName}
  />

  <Input
    label="What's needed?"
    placeholder="English tutors, Food donations, Drivers..."
    value={title}
  />

  <TextArea
    label="Description"
    placeholder="Tell us more about this need..."
    value={description}
  />

  <Input
    label="Location (optional)"
    placeholder="North Minneapolis, Downtown St. Paul..."
    value={location}
  />

  <Input
    label="Contact (optional)"
    placeholder="Phone, email, or website"
    value={contact}
  />

  <Select label="Urgency">
    <Option value="urgent">Urgent</Option>
    <Option value="normal">Normal</Option>
    <Option value="low">Low</Option>
  </Select>

  <Button onPress={handleSubmit}>Submit Need</Button>
</Form>
```

### Admin UI - Approval Queue

Shows both scraped and user-submitted:

```
┌─────────────────────────────────────────────────────┐
│ 🌐 English Tutors Needed                            │
│ Source: Arrive Ministries (scraped)                 │
│ TLDR: English tutors for refugee families           │
│ [Approve] [Edit] [Reject]                           │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│ 👤 Food Pantry Volunteers Needed                    │
│ Source: User submitted by volunteer #abc123         │
│ Location: North Minneapolis                          │
│ TLDR: Food pantry needs weekend volunteers          │
│ [Approve] [Edit] [Reject]                           │
└─────────────────────────────────────────────────────┘
```

## Benefits

✅ **Crowdsourced Discovery** - Volunteers report needs they see
✅ **Real-Time Updates** - No need to wait for website scrape
✅ **Community Knowledge** - Tap into local networks
✅ **Same Quality Control** - Human-in-the-loop approval for all sources
✅ **Spam Prevention** - IP tracking, admin review, duplicate detection

## Anti-Abuse Measures

1. **Human approval required** - All submissions go to pending_approval
2. **Content hash deduplication** - Detect duplicate submissions
3. **IP address tracking** - Identify spam patterns
4. **Rate limiting** (future) - Limit submissions per IP/volunteer
5. **Rejection logging** (future) - Track spam patterns

## Future Enhancements

- [ ] Upvoting/verification by other volunteers
- [ ] Automatic spam detection (ML model)
- [ ] Photo uploads for needs
- [ ] In-app messaging between submitter and organization
- [ ] Follow-up notifications ("Your submitted need was approved!")

"use client";

import * as React from "react";
import Link from "next/link";
import { Badge } from "@/components/ui/badge";
import { ExternalLink } from "lucide-react";
import { TagsSection } from "@/components/admin/TagsSection";
import { OrganizationRow } from "./rows/OrganizationRow";
import { LocationRow } from "./rows/LocationRow";
import { ContactsRow } from "./rows/ContactsRow";
import { HoursRow } from "./rows/HoursRow";
import { EventDateRow } from "./rows/EventDateRow";
import { LinkRow } from "./rows/LinkRow";
import { ItemsRow } from "./rows/ItemsRow";
import { PersonRow } from "./rows/PersonRow";
import { SourceAttributionRow } from "./rows/SourceAttributionRow";
import { StatusRow } from "./rows/StatusRow";

type Note = {
  id: string;
  content: string;
  severity: string;
  sourceUrl?: string | null;
  isPublic: boolean;
  createdBy?: string | null;
  expiredAt?: string | null;
  createdAt: string;
  linkedPosts?: Array<{ id: string; title: string }> | null;
};

type RightPost = {
  id: string;
  organizationId?: string | null;
  organization?: { id: string; name: string } | null;
  location?: string | null;
  zipCode?: string | null;
  latitude?: number | null;
  longitude?: number | null;
  contacts?: Array<{ id: string; contactType: string; contactValue: string; contactLabel?: string | null }> | null;
  schedule?: Array<{ id?: string | null; day: string; opens: string; closes: string }> | null;
  datetime?: { start?: string | null; end?: string | null; cost?: string | null; recurring?: boolean | null } | null;
  link?: { label?: string | null; url?: string | null; deadline?: string | null } | null;
  items?: Array<{ name: string; detail?: string | null }> | null;
  person?: { name?: string | null; role?: string | null; bio?: string | null; photoUrl?: string | null; quote?: string | null; photoMediaId?: string | null } | null;
  sourceAttribution?: { sourceName?: string | null; attribution?: string | null } | null;
  postStatus?: { state?: string | null; verified?: string | null } | null;
  tags?: Array<{ id: string; kind: string; value: string; displayName?: string | null; color?: string | null }> | null;
  revisionOfPostId?: string | null;
  translationOfId?: string | null;
  duplicateOfId?: string | null;
};

type Actions = {
  inlineUpdate: (input: Record<string, unknown>) => Promise<unknown>;
  addContact: (input: { contactType: string; contactValue: string; contactLabel?: string | null }) => Promise<unknown>;
  removeContact: (contactId: string) => Promise<unknown>;
  addSchedule: (input: { dayOfWeek: number; opensAt: string; closesAt: string }) => Promise<unknown>;
  deleteSchedule: (scheduleId: string) => Promise<unknown>;
  upsertLink: (input: { label: string | null; url: string | null; deadline: string | null }) => Promise<unknown>;
  upsertDatetime: (input: { start: string | null; end: string | null; cost: string | null; recurring: boolean }) => Promise<unknown>;
  upsertPerson: (input: { name: string | null; role: string | null; bio: string | null; photoUrl: string | null; quote: string | null; photoMediaId: string | null }) => Promise<unknown>;
  upsertItems: (items: Array<{ name: string; detail?: string | null }>) => Promise<unknown>;
  upsertSourceAttr: (input: { sourceName: string | null; attribution: string | null }) => Promise<unknown>;
  upsertStatus: (input: { state: string | null; verified: string | null }) => Promise<unknown>;
};

type TagsData = {
  applicableKinds: Array<{ slug: string; displayName: string; locked: boolean }>;
  allTagsByKind: Record<string, Array<{ id: string; value: string; displayName?: string | null; color?: string | null }>>;
  onAddTags: (kindSlug: string, newTags: Array<{ value: string; displayName: string }>) => Promise<void>;
  onRemoveTag: (tagId: string) => Promise<void>;
  disabled: boolean;
};

export function PostDetailRight({
  post,
  notes,
  actions,
  tagsData,
}: {
  post: RightPost;
  notes: Note[];
  actions: Actions;
  tagsData: TagsData;
}) {
  return (
    <div className="space-y-6">
      {/* Primary info — static by default, click to edit */}
      <section>
        <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
          Details
        </h3>
        <dl className="px-1">
          <OrganizationRow
            organizationId={post.organizationId ?? null}
            organizationName={post.organization?.name ?? null}
            onSave={(orgId) => actions.inlineUpdate({ organizationId: orgId })}
          />
          <LocationRow
            location={post.location ?? null}
            zipCode={post.zipCode ?? null}
            latitude={post.latitude ?? null}
            longitude={post.longitude ?? null}
            onSave={({ location, zipCode }) =>
              actions.inlineUpdate({ location, zipCode })
            }
          />
          <ContactsRow
            contacts={post.contacts ?? []}
            onAdd={actions.addContact}
            onRemove={actions.removeContact}
          />
          <HoursRow
            schedule={post.schedule ?? []}
            onAdd={actions.addSchedule}
            onDelete={actions.deleteSchedule}
          />
          <EventDateRow
            datetime={post.datetime ?? null}
            onSave={actions.upsertDatetime}
          />
          <LinkRow link={post.link ?? null} onSave={actions.upsertLink} />
          <ItemsRow items={post.items ?? []} onSave={actions.upsertItems} />
          <PersonRow person={post.person ?? null} onSave={actions.upsertPerson} />
          <SourceAttributionRow
            sourceAttribution={post.sourceAttribution ?? null}
            onSave={actions.upsertSourceAttr}
          />
          <StatusRow postStatus={post.postStatus ?? null} onSave={actions.upsertStatus} />
        </dl>
      </section>

      {/* Tags */}
      <TagsSection
        tags={post.tags ?? []}
        applicableKinds={tagsData.applicableKinds}
        allTagsByKind={tagsData.allTagsByKind}
        onRemoveTag={tagsData.onRemoveTag}
        onAddTags={tagsData.onAddTags}
        disabled={tagsData.disabled}
      />

      {/* Notes */}
      {notes.length > 0 && (
        <section>
          <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
            Notes ({notes.length})
          </h3>
          <div className="space-y-2">
            {notes.map((note) => {
              const isExpired = !!note.expiredAt;
              const severityVariant: "danger" | "warning" | "info" =
                note.severity === "urgent" ? "danger" :
                  note.severity === "notice" ? "warning" : "info";
              const otherLinks = (note.linkedPosts ?? []).filter((p) => p.id !== post.id);
              return (
                <div
                  key={note.id}
                  className={`p-3 rounded-lg border ${isExpired ? "border-border bg-secondary opacity-60" : "border-border bg-card"}`}
                >
                  <div className="flex items-center gap-2 mb-1">
                    <Badge variant={severityVariant}>{note.severity}</Badge>
                    {note.isPublic && <Badge variant="success">public</Badge>}
                    {isExpired && <Badge variant="secondary">expired</Badge>}
                    <span className="text-xs text-muted-foreground">
                      {note.createdBy} · {new Date(note.createdAt).toLocaleDateString()}
                    </span>
                  </div>
                  <p className="text-sm text-foreground">{note.content}</p>
                  {note.sourceUrl && (
                    <a href={note.sourceUrl} target="_blank" rel="noopener noreferrer" className="text-xs text-link hover:text-link-hover mt-1 inline-block">
                      Source <ExternalLink className="inline w-3 h-3 ml-0.5" />
                    </a>
                  )}
                  {otherLinks.length > 0 && (
                    <div className="flex flex-wrap items-center gap-1 mt-1.5">
                      <span className="text-xs text-muted-foreground">Also on:</span>
                      {otherLinks.map((p) => (
                        <Link
                          key={p.id}
                          href={`/admin/posts/${p.id}`}
                          className="text-xs px-1.5 py-0.5 bg-secondary text-secondary-foreground rounded hover:bg-accent hover:text-accent-foreground transition-colors truncate max-w-[200px]"
                          title={p.title}
                        >
                          {p.title}
                        </Link>
                      ))}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </section>
      )}

      {/* Lineage — only shown when this post is a revision/translation/duplicate */}
      {(post.revisionOfPostId || post.translationOfId || post.duplicateOfId) && (
        <section>
          <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
            Lineage
          </h3>
          <div className="space-y-1 text-sm px-1">
            {post.revisionOfPostId && (
              <div className="flex justify-between py-1">
                <span className="text-muted-foreground">Revision of</span>
                <Link href={`/admin/posts/${post.revisionOfPostId}`} className="text-link hover:text-link-hover text-xs font-mono truncate max-w-[180px]">
                  {post.revisionOfPostId}
                </Link>
              </div>
            )}
            {post.translationOfId && (
              <div className="flex justify-between py-1">
                <span className="text-muted-foreground">Translation of</span>
                <Link href={`/admin/posts/${post.translationOfId}`} className="text-link hover:text-link-hover text-xs font-mono truncate max-w-[180px]">
                  {post.translationOfId}
                </Link>
              </div>
            )}
            {post.duplicateOfId && (
              <div className="flex justify-between py-1">
                <span className="text-muted-foreground">Duplicate of</span>
                <Link href={`/admin/posts/${post.duplicateOfId}`} className="text-link hover:text-link-hover text-xs font-mono truncate max-w-[180px]">
                  {post.duplicateOfId}
                </Link>
              </div>
            )}
          </div>
        </section>
      )}
    </div>
  );
}
